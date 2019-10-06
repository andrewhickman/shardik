use std::pin::Pin;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tonic::{Code, Request, Response, Status, Streaming};

use crate::connection::ConnectionMap;
use shardik::api::*;
use shardik::resource::Resource;

pub struct LockService {
    connections: Arc<ConnectionMap>,
}

#[tonic::async_trait]
impl server::LockService for LockService {
    type LockStream = Pin<Box<dyn Stream<Item = Result<LockResponse, Status>> + Send + 'static>>;

    async fn lock(
        &self,
        req: Request<Streaming<LockRequest>>,
    ) -> Result<Response<Self::LockStream>, Status> {
        let stream = req.into_inner();
        let connections = self.connections.clone();

        let res = async_stream::try_stream! {
            futures::pin_mut!(stream);

            let id = match stream.next().await {
                Some(req) => {
                    let req = req?;
                    req.expect_acquire()?
                },
                None => return,
            };
            log::info!("Received acquire request for shard {}", id);
            let (mut connection, data) = match connections.begin(&id).await {
                Some(result) => result,
                None => Err(Status::new(Code::NotFound, "key not found"))?,
            };
            log::info!("Sending acquired response for shard {}", id);
            yield LockResponse {
                body: Some(lock_response::Body::Acquired(data)),
            };

            connection.wait().await.unwrap();
            log::info!("Sending release response for shard {}", id);
            yield LockResponse {
                body: Some(lock_response::Body::Release(id.clone())),
            };

            let data = match stream.next().await {
                Some(req) => {
                    let req = req?;
                    req.expect_released()?
                },
                None => Err(Status::new(Code::DataLoss, "shard not released"))?,
            };
            log::info!("Received released request for shard {}", id);
            connection.release(data).unwrap();

            if let Some(req) = stream.next().await {
                log::error!("unexpected message {:?}", req);
                Err(Status::new(Code::FailedPrecondition, "connection closed"))?;
            }
        };

        Ok(Response::new(Box::pin(res) as Self::LockStream))
    }
}

impl LockService {
    pub fn new<R: Resource>(resource: &R) -> Self {
        LockService {
            connections: Arc::new(ConnectionMap::new(resource)),
        }
    }
}
