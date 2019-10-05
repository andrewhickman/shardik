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

impl LockResponse {
    pub fn expect_acquired(self) -> Result<Data, Box<dyn std::error::Error>> {
        match self {
            LockResponse {
                body: Some(lock_response::Body::Acquired(data)),
            } => Ok(data),
            _ => Err("expected response to be `acquired`".into()),
        }
    }

    pub fn expect_release(self) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            LockResponse {
                body: Some(lock_response::Body::Release(id)),
            } => Ok(id),
            _ => Err("expected response to be `release`".into()),
        }
    }
}
