use shardik::api::Data;

pub struct ConnectionMap {}

pub struct Connection;

impl ConnectionMap {
    pub fn new() -> Self {
        ConnectionMap {}
    }

    pub async fn begin(&self, id: &str) -> (Connection, Data) {
        unimplemented!()
    }
}

impl Connection {
    pub async fn end(self, id: &str, data: Data) {
        unimplemented!()
    }
}