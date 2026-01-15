use std::collections::HashMap;

use crate::{
  cache_key::CacheKey,
  storage::{Address, StorageStrategy},
};

pub struct DequeStorage {
  data: HashMap<CacheKey, Address>,
}

impl StorageStrategy for DequeStorage {
  fn insert(
    &mut self,
    cache_key: crate::cache_key::CacheKey,
    address: super::Address,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.data.insert(cache_key, address);
    Ok(())
  }

  fn get(&self, cache_key: &crate::cache_key::CacheKey) -> Option<&super::Address> {
    self.data.get(&cache_key)
  }
}

impl Default for DequeStorage {
  fn default() -> Self {
    Self {
      data: HashMap::new(),
    }
  }
}
