use std::fs;
use std::path::{Path, PathBuf};

pub trait Resource {
    fn keys(&self) -> Vec<(String, String)>;

    fn get_shard_id(key: &str) -> String;
    fn perturb_key(key: &str) -> String;
    fn access(key: &str);
}

pub struct FileSystem {
    base: PathBuf,
}

impl FileSystem {
    pub fn new(base: PathBuf) -> Self {
        FileSystem { base }
    }
}

impl Resource for FileSystem {
    fn keys(&self) -> Vec<(String, String)> {
        let mut keys = Vec::with_capacity(32 * 256);
        for shard_id in 0..32 {
            let shard_id = shard_id.to_string();
            let dir = self.base.join(&shard_id);
            fs::create_dir_all(&dir).unwrap();
            for id in 0..256 {
                let id = id.to_string();
                fs::File::create(&dir.join(&id)).unwrap();
                keys.push((shard_id.clone(), id));
            }
        }
        keys
    }

    fn get_shard_id(key: &str) -> String {
        Path::new(key)
            .parent()
            .unwrap_or("".as_ref())
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn perturb_key(_key: &str) -> String {
        unimplemented!()
    }

    fn access(_key: &str) {
        unimplemented!()
    }
}
