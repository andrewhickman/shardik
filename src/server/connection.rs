use std::sync::Arc;

use crate::ServiceState;
use shardik::api::{LockRequest, LockResponse};

pub struct Connection {
    service_state: Arc<ServiceState>,
}

impl Connection {
    pub fn new(service_state: Arc<ServiceState>) -> Self {
        Connection { service_state }
    }

    pub async fn handle(&mut self, req: LockRequest) -> LockResponse {
        unimplemented!()
    }
}
