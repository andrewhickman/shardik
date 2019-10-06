use std::mem::replace;

use chashmap::CHashMap;
use futures::channel::oneshot;

use shardik::api::ShardData;
use shardik::resource::Resource;

pub struct ConnectionMap {
    map: CHashMap<String, Shard>,
}

enum Shard {
    /// No client currently owns this shard.
    Unlocked(ShardData),
    /// A client currently owns this shard. It can be stolen by sending a request to the
    /// `ConnectionSender`.
    Locked(ConnectionSender),
}

pub struct ConnectionReceiver {
    request_rx: Option<oneshot::Receiver<()>>,
    response_tx: Option<oneshot::Sender<ShardData>>,
}

struct ConnectionSender {
    request_tx: oneshot::Sender<()>,
    response_rx: oneshot::Receiver<ShardData>,
}

impl ConnectionMap {
    pub fn new<R: Resource>(resource: &R) -> Self {
        let map = CHashMap::new();
        for (shard_id, key) in resource.keys() {
            map.alter(shard_id, |shard| {
                let mut shard = shard.unwrap_or_default();
                match shard {
                    Shard::Unlocked(ref mut data) => data.locks.insert(key, false),
                    _ => unreachable!(),
                };
                Some(shard)
            });
        }
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
            request_rx: Some(request_rx),
            response_tx: Some(response_tx),
        };

        let prev_shard = {
            let mut shard = self.map.get_mut(id)?;
            replace(&mut *shard, Shard::Locked(cur_sender))
        };
        let data = match prev_shard {
            Shard::Unlocked(data) => data,
            Shard::Locked(prev_sender) => {
                log::info!("shard {} has existing lock, stealing", id);
                prev_sender.acquire().await
            }
        };

        Some((cur_receiver, data))
    }
}

impl Default for Shard {
    fn default() -> Self {
        Shard::Unlocked(ShardData::default())
    }
}

impl ConnectionReceiver {
    /// Wait until another client requests the shard
    pub async fn wait(&mut self) -> Result<(), oneshot::Canceled> {
        self.request_rx.take().expect("already waited").await
    }

    /// Send the shard data to the client which requested the shard
    pub fn release(&mut self, data: ShardData) -> Result<(), ShardData> {
        self.response_tx
            .take()
            .expect("already released")
            .send(data)
    }
}

impl ConnectionSender {
    /// Request the shard from another client, and wait for it to be returned.
    pub async fn acquire(self) -> ShardData {
        self.request_tx.send(()).unwrap();
        self.response_rx.await.unwrap()
    }
}
