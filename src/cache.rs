#![allow(unused)]

use std::{error::Error, io};

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::CacheKey,
  storage::{Address, Storage, StorageFlushStrategy, StorageStrategy},
};

pub struct GeoCache<S: StorageStrategy> {
  /// Cache configuration
  config: GeoCacheConfig,
  /// The active storage strategy (e.g., Deque, LRU)
  strategy: S,
  /// Underlying file storage, present only when persistence is enabled
  storage: Option<Storage>,
  /// Tracks the number of inserted records since the last flush
  record_counter: usize,
}

impl<S: StorageStrategy + Default> GeoCache<S> {
  pub fn new(config: GeoCacheConfig) -> Self {
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
      strategy: S::default(),
      storage,
      record_counter: 0,
    }
  }

  #[must_use = "use for persistence disk initialization"]
  pub fn init(&mut self) -> io::Result<()> {
    if let Some(storage) = &mut self.storage {
      self.strategy.read(storage);
    }

    Ok(())
  }

  /// Flushes data into the persistence storage
  fn flush_into_storage(&mut self) -> io::Result<()> {
    if let Some(storage) = self.storage.as_mut() {
      self.strategy.flush(storage);
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

  /// Inserts a new key into `GeoCache` data.
  /// The key consists of `(latitude, longitude, language_code)`.
  pub fn insert(
    &mut self,
    (lat, lng, lang): (f64, f64, &str),
    address: Address,
  ) -> Result<(), Box<dyn Error>> {
    self
      .strategy
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
    Ok(self.strategy.get(&CacheKey::try_new(lat, lng, lang)?))
  }
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;

  use crate::{
    cache::GeoCache,
    cache_config::{GeoCacheConfig, GeoCacheConfigBuilder},
    storage::deque::{DeqeueStorageItem, DequeStorage},
  };

  fn create_example_geo_cache_config() -> GeoCacheConfig {
    GeoCacheConfigBuilder::default().build()
  }

  fn create_example_deque_geo_cache() -> GeoCache<DequeStorage> {
    GeoCache::new(create_example_geo_cache_config())
  }

  fn create_example_deque_geo_cache_with_storage_path(
    tmp: Option<NamedTempFile>,
  ) -> (GeoCache<DequeStorage>, NamedTempFile) {
    let tmp = tmp.unwrap_or_else(|| NamedTempFile::new().unwrap());

    let mut geo_cache = GeoCache::new(
      GeoCacheConfigBuilder::default()
        .storage_file_path(tmp.path())
        .storage_flush_strategy(crate::StorageFlushStrategy::Immediately)
        .build(),
    );

    geo_cache.init();
    (geo_cache, tmp)
  }

  #[test]
  fn test_deque_cache_insert() {
    let (mut geo_cache, tmp) = create_example_deque_geo_cache_with_storage_path(None);

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "en"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(
      geo_cache.get((48.1645819, 17.1847104, "en")).unwrap(),
      Some(&"Bratislava, Slovakia".to_string())
    );

    drop(geo_cache);

    let (mut geo_cache, tmp) = create_example_deque_geo_cache_with_storage_path(Some(tmp));

    assert_eq!(
      geo_cache.get((48.1645819, 17.1847104, "en")).unwrap(),
      Some(&"Bratislava, Slovakia".to_string())
    );
  }

  #[test]
  fn test_deque_cache_get() {
    let mut geo_cache = create_example_deque_geo_cache();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "en"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(
      geo_cache.get((48.1645819, 17.1847104, "en")).unwrap(),
      Some(&"Bratislava, Slovakia".to_string())
    )
  }

  #[test]
  fn test_deque_cache_get_failed() {
    let mut geo_cache = create_example_deque_geo_cache();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "en"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(geo_cache.get((88.1645819, 17.1847104, "en")).unwrap(), None)
  }

  #[test]
  fn test_deque_cache_get_file_size() {
    let (mut geo_cache, tmp) = create_example_deque_geo_cache_with_storage_path(None);

    let bratislava_address = "Bratislava, Slovakia".to_string();
    let prague_address = "Prague, Czechia".to_string();

    let (b_len, p_len) = (bratislava_address.len(), prague_address.len());

    geo_cache
      .insert((48.1645819, 17.1847104, "en"), bratislava_address)
      .unwrap();

    geo_cache
      .insert((50.073658, 14.418540, "en"), prague_address)
      .unwrap();

    let storage = geo_cache.storage.as_ref().unwrap();

    assert_eq!(
      storage.len().unwrap(),
      (DeqeueStorageItem::key_len() * 2 + p_len + b_len) as u64
    )
  }
}
