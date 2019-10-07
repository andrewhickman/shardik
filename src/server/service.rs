use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};
use tokio::timer;
use tonic::{Code, Request, Response, Status, Streaming};

use crate::connection::{ConnectionMap, ConnectionReceiver};
use shardik::api::*;
use shardik::resource::Resource;

#[derive(Clone)]
pub struct LockService {
    connections: Arc<ConnectionMap>,
    latency: Duration,
}

#[tonic::async_trait]
impl server::LockService for LockService {
    type LockStream = mpsc::Receiver<Result<LockResponse, Status>>;

    async fn lock(
        &self,
        request: Request<Streaming<LockRequest>>,
    ) -> Result<Response<Self::LockStream>, Status> {
        let request_rx = request.into_inner();
        let (response_tx, response_rx) = mpsc::channel(0);

        tokio::spawn(LockService::lock_handle_error(
            self.clone(),
            request_rx,
            response_tx.clone(),
        ));
        Ok(Response::new(response_rx))
    }
}

impl LockService {
    pub fn new<R: Resource>(resource: &R, latency: Duration) -> Self {
        LockService {
            connections: Arc::new(ConnectionMap::new(resource)),
            latency,
        }
    }

    pub async fn lock_handle_error(
        self,
        request: impl Stream<Item = Result<LockRequest, Status>>,
        mut response: mpsc::Sender<Result<LockResponse, Status>>,
    ) {
        if let Err(status) = self.lock_inner(request, response.clone()).await {
            let _ = response.send(Err(status)).await;
        }
    }

    pub async fn lock_inner(
        self,
        request: impl Stream<Item = Result<LockRequest, Status>>,
        mut response: mpsc::Sender<Result<LockResponse, Status>>,
    ) -> Result<(), Status> {
        futures::pin_mut!(request);
        let latency = self.latency;

        let shard_id = match request.next().await {
            Some(req) => req?.expect_acquire()?,
            None => return Err(Status::new(Code::InvalidArgument, "expected request")),
        };
        log::info!("Received acquire request for shard {}", shard_id);
        let (connection, data) = match self.connections.begin(&shard_id).await {
            Some(result) => result,
            None => return Err(Status::new(Code::NotFound, "key not found")),
        };
        timer::delay_for(latency).await;
        log::info!("Sending acquired response for shard {}", shard_id);
        response
            .send(Ok(LockResponse {
                body: Some(lock_response::Body::Acquired(data)),
            }))
            .await
            .unwrap();

        tokio::spawn(ConnectionReceiver::request_release(
            connection.request_rx,
            move |shard_id| {
                async move {
                    timer::delay_for(latency).await;
                    log::info!("Sending release response for shard {}", shard_id);
                    let _ = response
                        .send(Ok(LockResponse {
                            body: Some(lock_response::Body::Release(shard_id)),
                        }))
                        .await;
                }
            },
        ));

        let data = match request.next().await {
            Some(req) => req?.expect_released()?,
            None => return Err(Status::new(Code::DataLoss, "shard not released")),
        };
        log::info!("Received released request for shard {}", shard_id);
        connection.response_tx.send(data).unwrap();

        // if let Some(req) = request.next().await {
        //     log::error!("unexpected message {:?}", req);
        //     return Err(Status::new(Code::FailedPrecondition, "connection closed"));
        // }

        Ok(())
    }
}
