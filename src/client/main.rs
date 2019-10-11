mod lock;
mod logger;
mod ui;

use std::sync::Arc;
use std::time::Duration;
use std::io;

use futures::StreamExt;
use structopt::StructOpt;
use tokio::net::signal;

use crate::lock::Lock;
use crate::ui::Ui;
use shardik::api::*;
use shardik::metrics::{Metrics, MetricsOpts};
use shardik::resource::{FileSystem, Resource};

#[derive(StructOpt)]
struct Opts {
    #[structopt(long, default_value = "http://[::1]:10000")]
    endpoint: http::Uri,
    #[structopt(flatten)]
    fs: FileSystem,
    #[structopt(flatten)]
    metrics: MetricsOpts,
    /// The name of the client.
    #[structopt(long)]
    client_name: Option<String>,
    /// The initial key to lock.
    #[structopt(long)]
    initial_key: String,
    /// The number of iterations to run for.
    #[structopt(long)]
    iterations: Option<u64>,
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
    logger::init();

    let screen = crossterm::AlternateScreen::to_alternate(true)?;
    let backend = tui::backend::CrosstermBackend::with_alternate_screen(io::stdout(), screen)?;
    let mut terminal = tui::Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let mut ui = Ui::new();

    let resource = Arc::new(opts.fs);
    let metrics = Metrics::new(opts.metrics)?;
    let client = client::LockServiceClient::connect(opts.endpoint)?;
    let mut lock = Lock::new(opts.client_name, client, resource.clone(), metrics);

    let mut key = opts.initial_key;

    let mut i = 0u64;
    let ctrl_c = signal::ctrl_c()?;
    futures::pin_mut!(ctrl_c);
    loop {
        if opts.iterations == Some(i) {
            log::info!("Completed {} iterations, exiting...", i);
            break;
        }
        if futures::poll!(ctrl_c.next()).is_ready() {
            log::info!("Received CTRL-C signal, exiting...");
            break;
        }

        ui.draw(&mut terminal, &lock)?;

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

        if opts.iterations.is_some() {
            i += 1;
        }
    }

    lock.release_all().await;

    Ok(())
}
