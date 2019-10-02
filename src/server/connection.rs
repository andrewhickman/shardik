use chashmap::CHashMap;
use futures::channel::oneshot;

use shardik::api::Data;

pub struct ConnectionMap {
    map: CHashMap<String, Shard>
}

enum Shard {
    Unlocked(Data),
    Locked(ConnectionSender)
}

pub struct ConnectionReceiver {
    request: Option<oneshot::Receiver<()>>,
    response: Option<oneshot::Sender<Data>>,
}

struct ConnectionSender {
    request: oneshot::Sender<()>,
    response: oneshot::Receiver<Data>,
}

impl ConnectionMap {
    pub fn new() -> Self {
        ConnectionMap {
            map: CHashMap::new(),
        }
    }

    pub async fn begin(&self, id: &str) -> (ConnectionReceiver, Data) {
        unimplemented!()
    }
}

impl ConnectionReceiver {
    pub async fn wait(&mut self) -> Result<(), oneshot::Canceled> {
        self.request.take().expect("already waited").await
    }

    pub fn release(&mut self, data: Data) -> Result<(), Data> {
        self.response.take().expect("already released").send(data)
    }
}

impl ConnectionSender {

}