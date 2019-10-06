use std::collections::hash_map::{self, HashMap};
use std::marker::PhantomData;
use std::mem::replace;
use std::sync::{Arc, Mutex};

use futures::channel::mpsc;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tonic::transport::Channel;
use tonic::{Request, Status};

use shardik::api::*;
use shardik::resource::Resource;

pub struct Lock<R> {
    client: client::LockServiceClient<Channel>,
    cache: HashMap<String, Arc<Mutex<Option<Data>>>>,
    _resource: PhantomData<R>,
}

impl<R: Resource> Lock<R> {
    pub fn new(client: client::LockServiceClient<Channel>) -> Self {
        Lock {
            client,
            cache: HashMap::new(),
            _resource: PhantomData,
        }
    }

    pub async fn lock(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        log::info!("Trying to lock key {}", key);
        self.set_locked(key, true).await
    }

    pub async fn unlock(&mut self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Unlocking key {}", key);
        assert!(self.set_locked(key, false).await?);
        Ok(())
    }

    async fn set_locked(
        &mut self,
        key: &str,
        value: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let set = |data: &mut Data| replace(data.claims.get_mut(key).unwrap(), value) != value;

        let shard_id = R::get_shard_id(key);
        if let hash_map::Entry::Occupied(entry) = self.cache.entry(shard_id.clone()) {
            {
                let mut lock = entry.get().lock().unwrap();
                if let Some(data) = lock.as_mut() {
                    return Ok(set(data));
                }
            }
            entry.remove_entry();
        }

        log::warn!("Acquiring new shard {}", shard_id);
        let (mut request_tx, request_rx) = mpsc::channel(0);
        let mut response_rx = self
            .client
            .lock(Request::new(request_rx))
            .await?
            .into_inner();

        request_tx
            .send(Ok(LockRequest {
                body: Some(lock_request::Body::Acquire(shard_id)),
            }))
            .await?;
        let mut data = response_rx.next().await.unwrap()?.expect_acquired()?;

        let result = set(&mut data);

        let data = Arc::new(Mutex::new(Some(data)));
        tokio::spawn(handle_release(request_tx, response_rx, data.clone()));

        Ok(result)
    }
}

async fn handle_release(
    request_tx: impl Sink<Result<LockRequest, Status>, Error = mpsc::SendError>,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
    data: Arc<Mutex<Option<Data>>>,
) {
    if let Err(err) = handle_release_inner(request_tx, response_rx, data).await {
        log::error!("Handle release failed: {}", err);
    }
}

async fn handle_release_inner(
    request_tx: impl Sink<Result<LockRequest, Status>, Error = mpsc::SendError>,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
    data: Arc<Mutex<Option<Data>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    futures::pin_mut!(request_tx);
    futures::pin_mut!(response_rx);
    let response = match response_rx.next().await {
        Some(response) => response?,
        None => return Err("lock not released".into()),
    };
    let shard_id = response.expect_release()?;
    log::warn!("shard {} stolen", shard_id);

    let data = data.lock().unwrap().take().unwrap();
    request_tx
        .send(Ok(LockRequest {
            body: Some(lock_request::Body::Released(data)),
        }))
        .await?;
    log::info!("sending released request for shard {}", shard_id);

    if let Some(res) = response_rx.next().await {
        log::error!("unexpected message {:?}", res);
        return Err("connection closed".into());
    }
    Ok(())
}
