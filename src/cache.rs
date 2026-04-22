#![allow(unused)]

use std::{error::Error, io, path::PathBuf};

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::CacheKey,
  storage::{Address, Storage, StorageFlushStrategy, StorageStrategy},
};

#[cfg(feature = "testing")]
use crate::storage::StorageStrategyWithCapacity;

pub struct GeoCache<S: StorageStrategy> {
  /// Cache configuration
  config: GeoCacheConfig,
  /// The active storage strategy (e.g., Deque, LRU)
  strategy: S,
  /// Underlying file storage, present only when persistence is enabled
  storage: Option<Storage>,
  /// Tracks the number of inserted records since the last flush
  pending_inserts: usize,
}

impl<S: StorageStrategy + Default> GeoCache<S> {
  fn create_storage(storage_file_path: Option<&PathBuf>) -> Option<Storage> {
    storage_file_path
      .as_ref()
      .and_then(|file_path| match Storage::try_new(file_path) {
        Ok(storage) => Some(storage),
        Err(err) => {
          println!("GeoCache storage error: {}", err);
          None
        }
      })
  }

  pub fn new(config: GeoCacheConfig) -> Self {
    let storage = GeoCache::<S>::create_storage(config.storage_file_path.as_ref());

    Self {
      config,
      strategy: S::default(),
      storage,
      pending_inserts: 0,
    }
  }

  #[cfg(feature = "testing")]
  pub fn with_capacity(config: GeoCacheConfig, capacity: usize) -> Self
  where
    S: StorageStrategyWithCapacity,
  {
    let storage = GeoCache::<S>::create_storage(config.storage_file_path.as_ref());

    Self {
      config,
      strategy: S::with_capacity(capacity),
      storage,
      pending_inserts: 0,
    }
  }

  #[must_use = "use for persistence disk initialization"]
  pub fn init(&mut self) -> io::Result<()> {
    if let Some(storage) = &mut self.storage {
      self.strategy.read(storage);
    }

    self.strategy.memory_max_size(self.config.memory_max_size);

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
      StorageFlushStrategy::RecordCount(count) => self.pending_inserts >= *count,
    }
  }

  fn flush_if_needed(&mut self) {
    if self.should_flush() {
      self.flush_into_storage();
      self.pending_inserts = 0;
    }
  }

  /// In a case when a new record data overflow limited memory max size
  /// delete oldest record by the provided strategy.
  fn evict_if_needed(&mut self, address_len: usize) -> io::Result<()> {
    if let Some(storage) = &mut self.storage {
      self.strategy.evict_if_needed(storage, address_len)?;
    }

    Ok(())
  }

  /// Inserts a new key into `GeoCache` data.
  /// The key consists of `(latitude, longitude, language_code)`.
  pub fn insert(
    &mut self,
    (lat, lng, lang): (f64, f64, &str),
    address: Address,
  ) -> Result<(), Box<dyn Error>> {
    let cache_key = CacheKey::try_new(lat, lng, lang)?;

    self.evict_if_needed(address.len())?;
    self.strategy.insert(cache_key, address)?;
    self.pending_inserts += 1;
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

  /// Returns the number of records currently held in memory.
  pub fn in_memory_record_count(&self) -> usize {
    self.strategy.in_memory_record_count()
  }
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;

  use crate::{
    StorageFlushStrategy,
    cache::GeoCache,
    cache_config::{GeoCacheConfig, GeoCacheConfigBuilder},
    storage::deque::{DequeStorage, DequeStorageItem},
  };

  fn create_example_geo_cache_config() -> GeoCacheConfig {
    GeoCacheConfigBuilder::default().build()
  }

  fn create_example_deque_geo_cache() -> GeoCache<DequeStorage> {
    GeoCache::new(create_example_geo_cache_config())
  }

  fn create_disk_cache_flush_immediately(
    tmp: Option<NamedTempFile>,
  ) -> (GeoCache<DequeStorage>, NamedTempFile) {
    let tmp = tmp.unwrap_or_else(|| NamedTempFile::new().unwrap());

    let mut geo_cache = GeoCache::<DequeStorage>::new(
      GeoCacheConfigBuilder::default()
        .storage_file_path(tmp.path())
        .storage_flush_strategy(StorageFlushStrategy::Immediately)
        .build(),
    );

    geo_cache.init();
    (geo_cache, tmp)
  }

  fn count_records_on_disk(tmp: &NamedTempFile) -> usize {
    let mut geo_cache = GeoCache::<DequeStorage>::new(
      GeoCacheConfigBuilder::default()
        .storage_file_path(tmp.path())
        .build(),
    );

    geo_cache.init();
    geo_cache.in_memory_record_count()
  }

  fn create_disk_cache_flush_every_5(
    tmp: Option<NamedTempFile>,
  ) -> (GeoCache<DequeStorage>, NamedTempFile) {
    let tmp = tmp.unwrap_or_else(|| NamedTempFile::new().unwrap());

    let mut geo_cache = GeoCache::<DequeStorage>::new(
      GeoCacheConfigBuilder::default()
        .storage_file_path(tmp.path())
        // Flush every 5th record
        .storage_flush_strategy(StorageFlushStrategy::RecordCount(5))
        .build(),
    );

    geo_cache.init();
    (geo_cache, tmp)
  }

  #[test]
  fn test_deque_cache_insert() {
    let (mut geo_cache, tmp) = create_disk_cache_flush_immediately(None);

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

    let (mut geo_cache, tmp) = create_disk_cache_flush_immediately(Some(tmp));

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
    let (mut geo_cache, tmp) = create_disk_cache_flush_immediately(None);

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
      (DequeStorageItem::len() * 2 + p_len + b_len) as u64
    )
  }

  #[test]
  fn test_deque_cache_insert_record_count_flush() {
    let (mut geo_cache, tmp) = create_disk_cache_flush_every_5(None);

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

    let records_on_disk = count_records_on_disk(&tmp);

    // Need to be 1 because in-meory after every insertion
    assert_eq!(geo_cache.in_memory_record_count(), 1);

    // Need to be zero because flushing every 5th
    assert_eq!(records_on_disk, 0);

    // Add another 8 records
    for i in 0..8 {
      geo_cache
        .insert(
          (48.1645819, 17.1847104, "en"),
          "Bratislava, Slovakia".to_string(),
        )
        .unwrap();
    }

    let records_on_disk = count_records_on_disk(&tmp);

    assert_eq!(geo_cache.in_memory_record_count(), 9);
    assert_eq!(records_on_disk, 5);

    geo_cache
      .insert((50.073658, 14.418540, "en"), "Prague, Czechia".to_string())
      .unwrap();

    let records_on_disk = count_records_on_disk(&tmp);

    assert_eq!(geo_cache.in_memory_record_count(), 10);
    assert_eq!(records_on_disk, 10);
  }
}
