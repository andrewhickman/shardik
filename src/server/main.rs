#![recursion_limit = "512"]

mod connection;
mod service;

use std::path::PathBuf;
use std::thread;
use std::io::Write;

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
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] <{}> {}",
                record.level(),
                thread::current().name().unwrap_or(""),
                record.args()
            )
        })
        .init();

    let addr = "[::1]:10000".parse().unwrap();
    log::info!("Listening on: {}", addr);

    let resource = FileSystem::new(opts.path);
    let svc = server::LockServiceServer::new(LockService::new(&resource));
    Server::builder().serve(addr, svc).await?;
    Ok(())
}
