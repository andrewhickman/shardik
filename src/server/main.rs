use std::pin::Pin;

use futures::{Stream, StreamExt};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

use shardik::api::*;

pub struct LockService {}

#[tonic::async_trait]
impl server::LockService for LockService {
    type LockStream = Pin<Box<dyn Stream<Item = Result<LockResponse, Status>> + Send + 'static>>;

    async fn lock(
        &self,
        req: Request<Streaming<LockRequest>>,
    ) -> Result<Response<Self::LockStream>, Status> {
        unimplemented!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "[::1]:10000".parse().unwrap();
    log::info!("Listening on: {}", addr);

    let svc = server::LockServiceServer::new(LockService {});
    Server::builder().serve(addr, svc).await?;
    Ok(())
}
