mod lock;

use std::sync::Arc;
use std::time::Duration;

use structopt::StructOpt;

use crate::lock::Lock;
use shardik::api::*;
use shardik::metrics::{Metrics, MetricsOpts};
use shardik::resource::{FileSystem, Resource};

#[derive(StructOpt)]
struct Opts {
    #[structopt(flatten)]
    fs: FileSystem,
    #[structopt(flatten)]
    metrics: MetricsOpts,
    /// The name of the client.
    #[structopt(long)]
    client_name: String,
    /// The initial key to lock.
    #[structopt(long)]
    initial_key: String,
    /// The number of iterations to run for.
    #[structopt(long)]
    iterations: u64,
    /// How long to lock keys for when accessing in milliseconds.
    #[structopt(long, default_value = "25")]
    access_duration: u64,
    /// The probability of switching shards when perturbing the key.
    #[structopt(long, default_value = "0.1")]
    perturb_shard_chance: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    env_logger::init();

    let resource = Arc::new(opts.fs);
    let metrics = Metrics::new(opts.metrics)?;
    let client = client::LockServiceClient::connect("http://[::1]:10000")?;
    let mut lock = Lock::new(opts.client_name, client, resource.clone(), metrics);

    let mut key = opts.initial_key;
    for _ in 0..opts.iterations {
        if lock.lock(&key).await? {
            log::info!("Lock acquired on key {}", key);
            resource
                .access(&key, Duration::from_millis(opts.access_duration))
                .await?;
            lock.unlock(&key).await?;
        } else {
            log::info!("Failed to lock key {}", key);
        }
        key = resource.perturb_key(&key, opts.perturb_shard_chance);
    }
    Ok(())
}
