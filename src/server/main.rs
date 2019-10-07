mod connection;
mod service;

use std::io::Write;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use structopt::StructOpt;
use tonic::transport::Server;

use crate::service::LockService;
use shardik::api::*;
use shardik::resource::FileSystem;

#[derive(StructOpt)]
struct Opts {
    #[structopt(long, default_value = "[::1]:10000")]
    endpoint: SocketAddr,
    #[structopt(flatten)]
    fs: FileSystem,
    /// The simulated latency of the service in milliseconds.
    #[structopt(long, default_value = "40")]
    latency: u64,
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

    log::info!("Listening on: {}", opts.endpoint);

    let resource = opts.fs;
    let svc = server::LockServiceServer::new(LockService::new(
        &resource,
        Duration::from_millis(opts.latency),
    ));
    Server::builder().serve(opts.endpoint, svc).await?;
    Ok(())
}
