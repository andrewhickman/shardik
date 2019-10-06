use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

use fs2::FileExt;
use rand::prelude::*;
use rand_distr::Poisson;
use tokio::timer;

#[tonic::async_trait]
pub trait Resource {
    fn keys(&self) -> Vec<(String, String)>;

    fn get_shard_id(key: &str) -> String;
    fn perturb_key(key: &str) -> String;
    async fn access(&self, key: &str) -> io::Result<()>;
}

pub struct FileSystem {
    base: PathBuf,
}

impl FileSystem {
    pub fn new(base: PathBuf) -> Self {
        FileSystem { base }
    }
}

#[tonic::async_trait]
impl Resource for FileSystem {
    fn keys(&self) -> Vec<(String, String)> {
        let mut keys = Vec::with_capacity(32 * 256);
        for shard_id in 0..SHARD_COUNT {
            let shard_id_str = shard_id.to_string();
            let dir = self.base.join(&shard_id_str);
            fs::create_dir_all(&dir).unwrap();

            for item_id in 0..ITEM_COUNT {
                let item_id_str = item_id.to_string();
                fs::File::create(&dir.join(&item_id_str)).unwrap();
                keys.push((shard_id_str.clone(), format_key(shard_id, item_id)));
            }
        }
        keys
    }

    fn get_shard_id(key: &str) -> String {
        parse_key(key).0.to_string()
    }

    fn perturb_key(key: &str) -> String {
        let (mut shard_id, mut item_id) = parse_key(key);

        let mut rng = rand::thread_rng();
        item_id = perturb(&mut rng, item_id, ITEM_COUNT);

        if rng.gen_bool(0.1) {
            shard_id = perturb(&mut rng, shard_id, SHARD_COUNT);
        }

        format_key(shard_id, item_id)
    }

    async fn access(&self, key: &str) -> io::Result<()> {
        let path = self.base.join(key);
        let file = fs::File::open(path)?;
        file.try_lock_exclusive()?;
        timer::delay_for(Duration::from_millis(25)).await;
        file.unlock()?;
        Ok(())
    }
}

const SHARD_COUNT: u32 = 32;
const ITEM_COUNT: u32 = 256;

fn format_key(shard_id: u32, item_id: u32) -> String {
    format!("{}/{}", shard_id, item_id)
}

fn parse_key(key: &str) -> (u32, u32) {
    let mut split = key.split("/");
    let shard_id = split.next().unwrap().parse().unwrap();
    let item_id = split.next().unwrap().parse().unwrap();
    (shard_id, item_id)
}

fn perturb(rng: &mut impl Rng, value: u32, max: u32) -> u32 {
    let distr = Poisson::new(4.0).unwrap();
    let abs_offset: u64 = distr.sample(rng);
    let offset = if rng.gen() {
        abs_offset as i32
    } else {
        -(abs_offset as i32)
    };

    (value as i32 + offset).rem_euclid(max as i32) as u32
}
