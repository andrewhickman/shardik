use chashmap::CHashMap;

use shardik::api::*;

pub struct CacheMap {
    map: CHashMap<String, Data>,
}

impl CacheMap {
    pub fn new() -> Self {
        CacheMap {
            map: CHashMap::new(),
        }
    }
}