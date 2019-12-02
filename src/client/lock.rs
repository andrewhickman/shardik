use std::collections::hash_map::{self, HashMap};
use std::mem::replace;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures::channel::mpsc;
use futures::future::join_all;
use futures::{SinkExt, Stream, StreamExt};
use tonic::transport::Channel;
use tonic::{Request, Status};

use shardik::api::*;
use shardik::metrics::Metrics;
use shardik::resource::Resource;

pub struct Lock<R> {
    client: client::LockServiceClient<Channel>,
    cache: HashMap<String, CacheEntry>,
    resource: Arc<R>,
    client_name: Option<String>,
    metrics: Metrics,
}

/// Represents cached shard data. If `data` is Some then it is cached. If it is none then
/// it has been recently stolen by another client.
#[derive(Debug, Clone)]
struct CacheEntry {
    data: Arc<Mutex<Option<ShardData>>>,
    request_tx: mpsc::Sender<Result<LockRequest, Status>>,
}

impl<R: Resource> Lock<R> {
    pub fn new(
        client_name: Option<String>,
        client: client::LockServiceClient<Channel>,
        resource: Arc<R>,
        metrics: Metrics,
    ) -> Self {
        Lock {
            client,
            cache: HashMap::new(),
            resource,
            client_name,
            metrics,
        }
    }

    pub async fn lock(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let start = Instant::now();
        let result = self.set_locked(key, true).await;
        self.metrics.log(&self.client_name, key, start.elapsed())?;
        result
    }

    pub async fn unlock(&mut self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        assert!(self.set_locked(key, false).await?);
        Ok(())
    }

    async fn set_locked(
        &mut self,
        key: &str,
        value: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let set = |data: &mut ShardData| replace(data.locks.get_mut(key).unwrap(), value) != value;

        let shard_id = self.resource.get_shard_id(key);
        if let hash_map::Entry::Occupied(entry) = self.cache.entry(shard_id.clone()) {
            {
                let mut lock = entry.get().data.lock().unwrap();
                if let Some(data) = lock.as_mut() {
                    // The shard is cached.
                    return Ok(set(data));
                }
            }
            // The cached shard was stolen by another thread, remove the entry.
            entry.remove_entry();
        }

        // Need to acquire the shard from the server.
        self.acquire(shard_id, set).await
    }

    async fn acquire(
        &mut self,
        shard_id: String,
        set: impl FnOnce(&mut ShardData) -> bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        log::warn!("Acquiring new shard {}", shard_id);
        let (mut request_tx, request_rx) = mpsc::channel(0);
        let mut response_rx = self
            .client
            .lock(Request::new(request_rx))
            .await?
            .into_inner();

        request_tx
            .send(Ok(LockRequest {
                body: Some(lock_request::Body::Acquire(shard_id.clone())),
            }))
            .await?;
        let mut data = response_rx.next().await.unwrap()?.expect_acquired()?;

        let result = set(&mut data);

        // Launch a background task to handle releasing the shard lock when requested by
        // the server.
        let cache_entry = CacheEntry {
            data: Arc::new(Mutex::new(Some(data))),
            request_tx,
        };
        tokio::spawn(handle_release(cache_entry.clone(), response_rx));
        self.cache.insert(shard_id, cache_entry);

        Ok(result)
    }

    pub async fn release_all(&mut self) {
        join_all(self.cache.drain().map(|(shard_id, cache_entry)| {
            async {
                futures::pin_mut!(shard_id);
                futures::pin_mut!(cache_entry);

                log::info!("sending released request for shard {}", shard_id);
                if let Err(err) = cache_entry.release().await {
                    log::error!("failed to release shard {}: {}", shard_id, err);
                }
            }
        }))
        .await;
    }
}

impl<R> Lock<R> {
    pub fn dump_shards<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.cache.iter().filter_map(|(shard_id, entry)| {
            if entry.data.lock().unwrap().is_some() {
                Some(shard_id.as_ref())
            } else {
                None
            }
        })
    }
}

impl CacheEntry {
    async fn release(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let data = match self.data.lock().unwrap().take() {
            Some(data) => data,
            None => return Ok(()),
        };
        self.request_tx
            .send(Ok(LockRequest {
                body: Some(lock_request::Body::Released(data)),
            }))
            .await?;
        Ok(())
    }
}

async fn handle_release(
    entry: CacheEntry,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
) {
    if let Err(err) = handle_release_inner(entry, response_rx).await {
        log::error!("Handle release failed: {}", err);
    }
}

/// Waits for the server to send a `Release` message on `response_rx` and then releases
/// the cached shard.
async fn handle_release_inner(
    entry: CacheEntry,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
) -> Result<(), Box<dyn std::error::Error>> {
    futures::pin_mut!(entry);
    futures::pin_mut!(response_rx);

    let response = match response_rx.next().await {
        Some(response) => response?,
        None => return Err("lock not released".into()),
    };
    let shard_id = response.expect_release()?;
    log::warn!("shard {} stolen", shard_id);

    log::info!("sending released request for shard {}", shard_id);
    entry.release().await?;

    if let Some(res) = response_rx.next().await {
        log::error!("unexpected message {:?}", res);
        return Err("connection closed".into());
    }
    Ok(())
}
