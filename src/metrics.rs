use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

use structopt::StructOpt;

#[derive(StructOpt)]
pub struct MetricsOpts {
    /// The file to write metrics to.
    #[structopt(long, parse(from_os_str))]
    metrics_file: PathBuf,
}

pub struct Metrics {
    writer: csv::Writer<fs::File>,
}

#[derive(serde::Serialize)]
pub struct Record<'a> {
    client_name: &'a str,
    key: &'a str,
    nanos: u128,
}

impl Metrics {
    pub fn new(opts: MetricsOpts) -> io::Result<Self> {
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(opts.metrics_file)?;
        let writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        Ok(Metrics {
            writer,
        })
    }

    pub fn log(&mut self, client_name: &str, key: &str, dur: Duration) -> csv::Result<()> {
        self.writer.serialize(Record {
            key,
            client_name,
            nanos: dur.as_nanos(),
        })
    }
}
