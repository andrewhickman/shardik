use std::mem::replace;

use chashmap::CHashMap;
use futures::channel::oneshot;

use shardik::api::Data;
use shardik::resource::Resource;

pub struct ConnectionMap {
    map: CHashMap<String, Shard>,
}

enum Shard {
    Unlocked(Data),
    Locked(ConnectionSender),
}

pub struct ConnectionReceiver {
    request_rx: Option<oneshot::Receiver<()>>,
    response_tx: Option<oneshot::Sender<Data>>,
}

struct ConnectionSender {
    request_tx: oneshot::Sender<()>,
    response_rx: oneshot::Receiver<Data>,
}

impl ConnectionMap {
    pub fn new<R: Resource>(resource: &R) -> Self {
        let map = CHashMap::new();
        for (shard_id, key) in resource.keys() {
            map.alter(shard_id, |shard| {
                let mut shard = shard.unwrap_or_default();
                match shard {
                    Shard::Unlocked(ref mut data) => data.claims.insert(key, false),
                    _ => unreachable!(),
                };
                Some(shard)
            });
        }
        ConnectionMap { map }
    }

    pub async fn begin(&self, id: &str) -> Option<(ConnectionReceiver, Data)> {
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

        let mut shard = self.map.get_mut(id)?;
        let data = match replace(&mut *shard, Shard::Locked(cur_sender)) {
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
        Shard::Unlocked(Data::default())
    }
}

impl ConnectionReceiver {
    pub async fn wait(&mut self) -> Result<(), oneshot::Canceled> {
        self.request_rx.take().expect("already waited").await
    }

    pub fn release(&mut self, data: Data) -> Result<(), Data> {
        self.response_tx
            .take()
            .expect("already released")
            .send(data)
    }
}

impl ConnectionSender {
    pub async fn acquire(self) -> Data {
        self.request_tx.send(()).unwrap();
        self.response_rx.await.unwrap()
    }
}
