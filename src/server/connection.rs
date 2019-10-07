use std::collections::HashMap;
use std::mem::replace;

use chashmap::CHashMap;
use futures::channel::oneshot;
use futures::Future;

use shardik::api::ShardData;
use shardik::resource::Resource;

pub struct ConnectionMap {
    map: CHashMap<String, ConnectionSender>,
}

pub struct ConnectionReceiver {
    pub request_rx: oneshot::Receiver<String>,
    pub response_tx: oneshot::Sender<ShardData>,
}

struct ConnectionSender {
    request_tx: oneshot::Sender<String>,
    response_rx: oneshot::Receiver<ShardData>,
}

impl ConnectionMap {
    pub fn new<R: Resource>(resource: &R) -> Self {
        let mut map = HashMap::<String, ShardData>::new();
        for (shard_id, key) in resource.keys() {
            map.entry(shard_id).or_default().locks.insert(key, false);
        }

        let map = map
            .into_iter()
            .map(|(k, v)| (k, ConnectionSender::from_data(v)))
            .collect();

        ConnectionMap { map }
    }

    /// Gets a shard with the given id, returning the shard data and a `ConnectionReceiver` to
    /// listen to to know when to release the shard.
    pub async fn begin(&self, id: &str) -> Option<(ConnectionReceiver, ShardData)> {
        let (request_tx, request_rx) = oneshot::channel();
        let (response_tx, response_rx) = oneshot::channel();
        let cur_sender = ConnectionSender {
            request_tx,
            response_rx,
        };
        let cur_receiver = ConnectionReceiver {
            request_rx,
            response_tx,
        };

        let prev_sender = {
            let mut shard = self.map.get_mut(id)?;
            replace(&mut *shard, cur_sender)
        };
        let data = prev_sender.acquire(id.to_owned()).await;

        Some((cur_receiver, data))
    }
}

impl ConnectionReceiver {
    pub async fn request_release<F, T>(request_rx: oneshot::Receiver<String>, release: F)
    where
        F: FnOnce(String) -> T,
        T: Future,
    {
        if let Ok(shard_id) = request_rx.await {
            release(shard_id).await;
        }
    }
}

impl ConnectionSender {
    fn from_data(data: ShardData) -> Self {
        let (request_tx, _) = oneshot::channel();
        let (response_tx, response_rx) = oneshot::channel();

        response_tx.send(data).unwrap();

        ConnectionSender {
            request_tx,
            response_rx,
        }
    }

    /// Request the shard from another client, and wait for it to be returned.
    pub async fn acquire(self, id: String) -> ShardData {
        let _ = self.request_tx.send(id);
        self.response_rx.await.unwrap()
    }
}
