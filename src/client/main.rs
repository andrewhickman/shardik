mod cache;
mod lock;

use std::path::PathBuf;

use structopt::StructOpt;

use shardik::api::*;
use shardik::resource::FileSystem;
use crate::lock::Lock;

#[derive(StructOpt)]
struct Opts {
    #[structopt(long, parse(from_os_str))]
    path: PathBuf,
    #[structopt(long)]
    key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    env_logger::init();

    let resource = FileSystem::new(opts.path);
    let client = client::LockServiceClient::connect("http://[::1]:10000")?;

    let lock = Lock::new(client);

    Ok(())
}
