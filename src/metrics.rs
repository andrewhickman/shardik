use std::borrow::Cow;
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Record<'a> {
    pub client_name: Cow<'a, str>,
    pub key: Cow<'a, str>,
    pub nanos: u128,
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
        Ok(Metrics { writer })
    }

    pub fn log(&mut self, client_name: &str, key: &str, dur: Duration) -> csv::Result<()> {
        self.writer.serialize(Record {
            key: Cow::Borrowed(key),
            client_name: Cow::Borrowed(client_name),
            nanos: dur.as_nanos(),
        })
    }
}

pub fn parse_records(opts: MetricsOpts) -> io::Result<Vec<Record<'static>>> {
    let file = fs::File::open(opts.metrics_file)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);
    Ok(reader
        .deserialize::<Record>()
        .take_while(Result::is_ok)
        .map(Result::unwrap)
        .collect())
}
