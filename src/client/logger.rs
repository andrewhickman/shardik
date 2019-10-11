use std::iter;

use crossbeam_queue::SegQueue;
use env_logger::filter::{self, Filter};
use env_logger::DEFAULT_FILTER_ENV;
use lazy_static::lazy_static;
use log::Log;

lazy_static! {
    static ref LOGGER: Logger = Logger::new();
}

pub fn init() {
    log::set_logger(&*LOGGER).unwrap();
    log::set_max_level(LOGGER.filter.filter());
}

pub fn drain() -> impl Iterator<Item = String> {
    iter::from_fn(|| LOGGER.queue.pop().ok())
}

struct Logger {
    queue: SegQueue<String>,
    filter: Filter,
}

impl Logger {
    fn new() -> Self {
        Logger {
            queue: SegQueue::new(),
            filter: filter::Builder::from_env(DEFAULT_FILTER_ENV).build(),
        }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.filter.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if self.filter.matches(record) {
            self.queue
                .push(format!("{}: {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}
