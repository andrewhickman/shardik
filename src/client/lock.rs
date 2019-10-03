use std::sync::Arc;

use tonic::transport::Channel;

use shardik::api::*;
use crate::cache::CacheMap;

pub struct Lock {
    client: client::LockServiceClient<Channel>,
    connections: Arc<CacheMap>,
}

impl Lock {
    pub fn new(client: client::LockServiceClient<Channel>) -> Self {
        Lock { 
            client,
            connections: Arc::new(CacheMap::new()),
        }
    }

    pub async fn lock(&mut self, key: String) -> bool {
        unimplemented!()
    }

    pub async fn unlock(&mut self, key: String) {
        unimplemented!()
    }
}