use structopt::StructOpt;

use shardik::metrics::{self, MetricsOpts, Record};

#[derive(StructOpt)]
struct Opts {
    #[structopt(flatten)]
    metrics: MetricsOpts,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();

    let records = metrics::parse_records(opts.metrics)?;
    println!("Parsed {} entries", records.len());
    if records.is_empty() {
        return Ok(());
    }

    let min_max: stats::MinMax<f64> = nanos(&records).collect();
    println!("min:\t{:.0} ns", min_max.min().unwrap());
    println!("max:\t{:.0} ns", min_max.max().unwrap());
    println!("mean:\t{:.0} ns", stats::mean(nanos(&records)));
    println!("median:\t{:.0} ns", stats::median(nanos(&records)).unwrap());

    Ok(())
}

fn nanos<'a>(records: &'a [Record<'a>]) -> impl Iterator<Item = f64> + 'a {
    records.iter().map(|record| record.nanos as f64)
}
