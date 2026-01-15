#![allow(unused)]

use std::{error::Error, io};

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::CacheKey,
  storage::{Address, Storage, StorageFlushStrategy, StorageStrategy},
};

pub struct GeoCache<StorageStrategy> {
  /// Configuration file
  config: GeoCacheConfig,

  /// Storage strategy (dequeue, LRU)
  data: StorageStrategy,

  /// Storage (file handler)
  /// Optioned, used just for persistance disk operations
  storage: Option<Storage>,

  /// Counts added records
  /// Used for flash into a persistance disk
  record_counter: usize,
}

impl<S: StorageStrategy + Default> GeoCache<S> {
  pub fn new(config: GeoCacheConfig) -> Self {
    // Tries to open a `storage file`.
    let storage =
      config
        .storage_file_path
        .as_ref()
        .and_then(|file_path| match Storage::try_new(file_path) {
          Ok(storage) => Some(storage),
          Err(err) => {
            println!("GeoCache storage error: {}", err);
            None
          }
        });

    Self {
      config,
      data: S::default(),
      storage,
      record_counter: 0,
    }
  }

  /// Flushs data into the persistance storage
  fn flush_into_storage(&mut self) -> io::Result<()> {
    if let Some(storage) = self.storage.as_mut() {
      self.data.flush(storage);
    } else {
      println!("GeoCache storage error: Trying to flush into a disk but storage is not loaded");
    }

    Ok(())
  }

  fn should_flush(&self) -> bool {
    match &self.config.storage_flush_strategy {
      StorageFlushStrategy::Never => false,
      StorageFlushStrategy::Immediately => true,
      StorageFlushStrategy::RecordCount(count) => self.record_counter >= *count,
    }
  }

  fn flush_if_needed(&mut self) {
    if self.should_flush() {
      self.flush_into_storage();
    }
  }

  // pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
  //   let file = File::open(&self.config.file_path)?;
  //   let json: HashMap<CacheKeyRaw, Address> = serde_json::from_reader(file)?;
  //
  //   println!("{:?}", json);
  //
  //   Ok(())
  //
  //   // let file = File::open(path)
  // }

  /// Inserts a new key into `GeoCache` data.
  /// The key consists of `(latitude, longitude, language_code)`.
  pub fn insert(
    &mut self,
    (lat, lng, lang): (f64, f64, &str),
    address: Address,
  ) -> Result<(), Box<dyn Error>> {
    self
      .data
      .insert(CacheKey::try_new(lat, lng, lang)?, address)?;

    self.flush_if_needed();

    Ok(())
  }

  /// Retrieves a decoded address from a key.
  /// The key consists of `(latitude, longitude, language_code)`.
  pub fn get(
    &self,
    (lat, lng, lang): (f64, f64, &str),
  ) -> Result<Option<&Address>, Box<dyn Error>> {
    Ok(self.data.get(&CacheKey::try_new(lat, lng, lang)?))
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    cache::GeoCache,
    cache_config::{GeoCacheConfig, GeoCacheConfigBuilder},
    storage::deque::DequeStorage,
  };

  fn create_example_geo_cache_config() -> GeoCacheConfig {
    GeoCacheConfigBuilder::default().build()
  }

  fn create_example_deque_geo_cache() -> GeoCache<DequeStorage> {
    GeoCache::new(create_example_geo_cache_config())
  }

  #[test]
  fn test_deque_cache_get() {
    let mut geo_cache = create_example_deque_geo_cache();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "sk"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(
      geo_cache.get((48.1645819, 17.1847104, "sk")).unwrap(),
      Some(&"Bratislava, Slovakia".to_string())
    )
  }

  #[test]
  fn test_deque_cache_get_failed() {
    let mut geo_cache = create_example_deque_geo_cache();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "sk"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(geo_cache.get((88.1645819, 17.1847104, "sk")).unwrap(), None)
  }
}
