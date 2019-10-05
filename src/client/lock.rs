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
    cache: Arc<Mutex<HashMap<String, Data>>>,
    _resource: PhantomData<R>,
}

impl<R: Resource> Lock<R> {
    pub fn new(client: client::LockServiceClient<Channel>) -> Self {
        Lock {
            client,
            cache: Arc::default(),
            _resource: PhantomData,
        }
    }

    pub async fn lock(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.set_locked(key, true).await
    }

    pub async fn unlock(&mut self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        self.set_locked(key, false).await
    }

    async fn set_locked(
        &mut self,
        key: &str,
        value: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let shard_id = R::get_shard_id(key);
        let mut cache = self.cache.lock().unwrap();
        let mut entry = cache.entry(shard_id.clone());
        let mut data = match entry {
            hash_map::Entry::Occupied(ref mut entry) => entry.get_mut(),
            hash_map::Entry::Vacant(mut entry) => {
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

                tokio::spawn(handle_release(request_tx, response_rx, self.cache.clone()));

                entry.insert(data)
            }
        };
        Ok(replace(data.claims.get_mut(key).unwrap(), value) != value)
    }
}

async fn handle_release(
    request_tx: impl Sink<Result<LockRequest, Status>, Error = mpsc::SendError>,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
    cache: Arc<Mutex<HashMap<String, Data>>>,
) {
    if let Err(err) = handle_release_inner(request_tx, response_rx, cache).await {
        log::error!("Handle release failed: {}", err);
    }
}

async fn handle_release_inner(
    request_tx: impl Sink<Result<LockRequest, Status>, Error = mpsc::SendError>,
    response_rx: impl Stream<Item = Result<LockResponse, Status>>,
    cache: Arc<Mutex<HashMap<String, Data>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    futures::pin_mut!(request_tx);
    futures::pin_mut!(response_rx);
    let response = match response_rx.next().await {
        Some(response) => response?,
        None => return Err("lock not released".into()),
    };
    let shard_id = response.expect_release()?;

    let data = cache.lock().unwrap().remove(&shard_id).unwrap();
    request_tx
        .send(Ok(LockRequest {
            body: Some(lock_request::Body::Released(data)),
        }))
        .await?;

    if let Some(response) = response_rx.next().await {
        return Err("connection closed".into());
    }
    Ok(())
}
