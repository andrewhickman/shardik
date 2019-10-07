use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::{env, io, thread};

use criterion::{criterion_group, criterion_main, Criterion};
use futures::future;
use tokio::net::process::Command as AsyncCommand;
use tokio::runtime::Runtime;

pub const SHARD_COUNT: u32 = 32;
pub const CONCURRENT_CLIENTS: u32 = 24;
pub const ITERATIONS: u32 = 1024;

pub fn multi_client_benchmark(c: &mut Criterion) {
    let mut server = Command::new(bin_path("server").unwrap())
        .arg("--shard-count")
        .arg(format!("{}", SHARD_COUNT))
        .arg("--latency")
        .arg("0")
        .spawn()
        .unwrap();
    // Wait for server to spin up
    thread::sleep(Duration::from_secs(1));

    let rt = Runtime::new().unwrap();
    c.bench_function("multi_client", |b| {
        b.iter(|| {
            rt.block_on(future::try_join_all(
                (0..CONCURRENT_CLIENTS).map(run_client),
            ))
            .unwrap()
        })
    });

    server.kill().unwrap();
}

async fn run_client(id: u32) -> Result<(), Box<dyn std::error::Error>> {
    let status = AsyncCommand::new(bin_path("client").unwrap())
        .arg("--shard-count")
        .arg(format!("{}", SHARD_COUNT))
        .arg("--client-name")
        .arg(format!("client-{}", id))
        .arg("--initial-key")
        .arg(format!("{}/{}", id % SHARD_COUNT, id / SHARD_COUNT))
        .arg("--iterations")
        .arg(format!("{}", ITERATIONS))
        .arg("--access-duration")
        .arg("0")
        .status()
        .await?;
    if !status.success() {
        return Err(format!("client exited unsuccessfully ({})", status).into());
    }
    Ok(())
}

fn bin_path(name: impl AsRef<Path>) -> io::Result<PathBuf> {
    let mut result = env::current_exe()?;
    result.pop();
    if result.ends_with("deps") {
        result.pop();
    }
    result.push(name);
    result.set_extension(env::consts::EXE_EXTENSION);
    Ok(result)
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = multi_client_benchmark
);
criterion_main!(benches);
