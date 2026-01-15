#![allow(unused)]

use std::error::Error;

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::CacheKey,
  storage::{Address, StorageStrategy},
};

pub struct GeoCache<StorageStrategy> {
  config: GeoCacheConfig,
  data: StorageStrategy,
}

impl<S: StorageStrategy + Default> GeoCache<S> {
  pub fn new(config: GeoCacheConfig) -> Self {
    Self {
      config,
      data: S::default(),
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
      .insert(CacheKey::try_new(lat, lng, lang)?, address)
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
