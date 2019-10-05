#![recursion_limit = "256"]

mod connection;
mod service;

use std::path::PathBuf;

use structopt::StructOpt;
use tonic::transport::Server;

use crate::service::LockService;
use shardik::api::*;
use shardik::resource::FileSystem;

#[derive(StructOpt)]
struct Opts {
    #[structopt(long, parse(from_os_str))]
    path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    env_logger::init();

    let addr = "[::1]:10000".parse().unwrap();
    log::info!("Listening on: {}", addr);

    let resource = FileSystem::new(opts.path);
    let svc = server::LockServiceServer::new(LockService::new(&resource));
    Server::builder().serve(addr, svc).await?;
    Ok(())
}
