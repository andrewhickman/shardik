#![recursion_limit="256"]

mod connection;
mod repository;

use std::pin::Pin;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status, Streaming};

use crate::connection::ConnectionMap;
use crate::repository::Repository;
use shardik::api::*;

pub struct LockService {
    state: Arc<ServiceState>,
}

pub struct ServiceState {
    repository: Repository,
    connections: ConnectionMap,
}

#[tonic::async_trait]
impl server::LockService for LockService {
    type LockStream = Pin<Box<dyn Stream<Item = Result<LockResponse, Status>> + Send + 'static>>;

    async fn lock(
        &self,
        req: Request<Streaming<LockRequest>>,
    ) -> Result<Response<Self::LockStream>, Status> {
        log::info!("Lock request: {:?}", req);

        let stream = req.into_inner();
        let state = self.state.clone();

        let res = async_stream::try_stream! {
            futures::pin_mut!(stream);

            let id = match stream.next().await {
                Some(req) => {
                    let req = req?;
                    req.expect_acquire()?
                },
                None => return,
            };
            let (connection, data) = state.connections.begin(&id).await;
            yield LockResponse {
                body: Some(lock_response::Body::Acquired(data)),
            };

            let data = match stream.next().await {
                Some(req) => {
                    let req = req?;
                    req.expect_released()?
                },
                None => Err(Status::new(Code::DataLoss, "shard not released"))?,
            };
            connection.end(&id, data).await;

            while let Some(req) = stream.next().await {
                Err(Status::new(Code::FailedPrecondition, "connection closed"))?;
            }
        };

        Ok(Response::new(Box::pin(res) as Self::LockStream))
    }
}

impl ServiceState {
    fn new() -> Self {
        ServiceState {
            repository: Repository::new(),
            connections: ConnectionMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "[::1]:10000".parse().unwrap();
    log::info!("Listening on: {}", addr);

    let svc = server::LockServiceServer::new(LockService {
        state: Arc::new(ServiceState::new()),
    });
    Server::builder().serve(addr, svc).await?;
    Ok(())
}
