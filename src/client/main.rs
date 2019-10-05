mod lock;

use std::path::PathBuf;

use structopt::StructOpt;

use crate::lock::Lock;
use shardik::api::*;
use shardik::resource::FileSystem;

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

    let lock: Lock<FileSystem> = Lock::<FileSystem>::new(client);

    Ok(())
}
