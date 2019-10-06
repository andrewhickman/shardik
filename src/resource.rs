use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

use fs2::FileExt;
use rand::prelude::*;
use rand_distr::Poisson;
use structopt::StructOpt;
use tokio::timer;

#[tonic::async_trait]
pub trait Resource {
    fn keys(&self) -> Vec<(String, String)>;
    fn get_shard_id(&self, key: &str) -> String;
    fn perturb_key(&self, key: &str, perturb_shard_chance: f64) -> String;
    async fn access(&self, key: &str, access_duration: Duration) -> io::Result<()>;
}

#[derive(StructOpt)]
pub struct FileSystem {
    /// The base directory to create files in.
    #[structopt(long, parse(from_os_str))]
    base_path: PathBuf,
    /// The number of shards to create.
    #[structopt(long, default_value = "32")]
    shard_count: u32,
    /// The number of files to create per shard.
    #[structopt(long, default_value = "128")]
    item_count: u32,
}

#[tonic::async_trait]
impl Resource for FileSystem {
    fn keys(&self) -> Vec<(String, String)> {
        let mut keys = Vec::with_capacity((self.shard_count * self.item_count) as usize);
        for shard_id in 0..self.shard_count {
            let shard_id_str = shard_id.to_string();
            let dir = self.base_path.join(&shard_id_str);
            fs::create_dir_all(&dir).unwrap();

            for item_id in 0..self.item_count {
                let item_id_str = item_id.to_string();
                fs::File::create(&dir.join(&item_id_str)).unwrap();
                keys.push((shard_id_str.clone(), format_key(shard_id, item_id)));
            }
        }
        keys
    }

    fn get_shard_id(&self, key: &str) -> String {
        parse_key(key).0.to_string()
    }

    fn perturb_key(&self, key: &str, perturb_shard_chance: f64) -> String {
        let (mut shard_id, mut item_id) = parse_key(key);

        let mut rng = rand::thread_rng();
        item_id = perturb(&mut rng, item_id, 4.0, self.item_count);

        if rng.gen_bool(perturb_shard_chance) {
            shard_id = perturb(&mut rng, shard_id, 0.5, self.shard_count);
        }

        format_key(shard_id, item_id)
    }

    async fn access(&self, key: &str, access_duration: Duration) -> io::Result<()> {
        let path = self.base_path.join(key);
        let file = fs::File::open(path)?;
        file.try_lock_exclusive()?;
        timer::delay_for(access_duration).await;
        file.unlock()?;
        Ok(())
    }
}

fn format_key(shard_id: u32, item_id: u32) -> String {
    format!("{}/{}", shard_id, item_id)
}

fn parse_key(key: &str) -> (u32, u32) {
    let mut split = key.split("/");
    let shard_id = split.next().unwrap().parse().unwrap();
    let item_id = split.next().unwrap().parse().unwrap();
    (shard_id, item_id)
}

fn perturb(rng: &mut impl Rng, value: u32, lambda: f64, max: u32) -> u32 {
    let distr = Poisson::new(lambda).unwrap();
    let abs_offset: u64 = distr.sample(rng);
    let offset = if rng.gen() {
        1 + abs_offset as i32
    } else {
        -(1 + abs_offset as i32)
    };

    (value as i32 + offset).rem_euclid(max as i32) as u32
}
