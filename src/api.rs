use tonic::{Code, Status};

tonic::include_proto!("shardik");

impl LockRequest {
    pub fn expect_acquire(self) -> Result<String, Status> {
        match self {
            LockRequest {
                body: Some(lock_request::Body::Acquire(id)),
            } => Ok(id),
            _ => Err(Status::new(
                Code::FailedPrecondition,
                "expected request to be `acquire`",
            )),
        }
    }

    pub fn expect_released(self) -> Result<Data, Status> {
        match self {
            LockRequest {
                body: Some(lock_request::Body::Released(data)),
            } => Ok(data),
            _ => Err(Status::new(
                Code::FailedPrecondition,
                "expected request to be `released`",
            )),
        }
    }
}
