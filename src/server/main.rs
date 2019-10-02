mod connection;
mod repository;

use std::pin::Pin;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

use crate::connection::Connection;
use crate::repository::Repository;
use shardik::api::*;

pub struct LockService {
    state: Arc<ServiceState>,
}

pub struct ServiceState {
    repository: Repository,
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
        let mut connection = Connection::new(self.state.clone());

        let res = async_stream::try_stream! {
            futures::pin_mut!(stream);

            while let Some(op) = stream.next().await {
                let op = op?;
                yield connection.handle(op).await;
            }
        };

        Ok(Response::new(Box::pin(res) as Self::LockStream))
    }
}

impl ServiceState {
    fn new() -> Self {
        ServiceState {
            repository: Repository::new(),
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
