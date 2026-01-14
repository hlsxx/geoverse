use std::{
  collections::HashMap,
  error::Error,
  fs::File,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
  errors::CountryCodeError,
  geo::{convert_coords_into_microdeg, convert_lang_to_u16},
  throw_country_code_error,
};

type Address = String;

#[derive(Debug, Serialize, Hash, Eq, PartialEq)]
struct CacheKeyRaw([u8; 20]);

impl<'de> Deserialize<'de> for CacheKeyRaw {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    if s.len() != 20 {
      return Err(serde::de::Error::custom(format!(
        "Expected 20 chars, got {}",
        s.len()
      )));
    }

    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(s.as_bytes());

    Ok(CacheKeyRaw(bytes))
  }
}

impl CacheKeyRaw {}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct CacheKey {
  lat: i32,
  lng: i32,
  lang: u16,
}

impl CacheKey {
  /// Creates new `CacheKey` from coordinates `lat, lng` and provided language `lang`
  ///
  /// # Arguments
  ///
  /// * `lat` - Coordinate latitude
  /// * `lng` - Coordinate longitude
  /// * `lang` - Language of a decoded address
  ///
  /// # Examples
  ///
  /// ```
  /// use geoverse::cache::CacheKey;
  /// let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk");
  /// ```
  ///
  /// # Errors
  ///
  /// * Provided lang and lat must fit in specific range
  /// * Provided language must be correct country code size of 2
  pub fn try_new(lat: f64, lng: f64, lang: &str) -> Result<Self, Box<dyn Error>> {
    let (lat, lng) = convert_coords_into_microdeg(lat, lng)?;
    let lang_as_u16 = convert_lang_to_u16(lang)?;

    Ok(Self {
      lat,
      lng,
      lang: lang_as_u16,
    })
  }
}

impl Into<CacheKeyRaw> for CacheKey {
  fn into(self) -> CacheKeyRaw {
    let s = format!("{};{};{}", self.lang, self.lat, self.lng);
    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(s.as_bytes());
    CacheKeyRaw(bytes)
  }
}

#[derive(Default)]
pub struct GeoCacheConfigBuilder {
  file_path: Option<PathBuf>,
  memory_max_size: Option<usize>,
  disk_max_size: Option<usize>,
}

impl GeoCacheConfigBuilder {
  pub fn file_path(mut self, file_path: impl Into<PathBuf>) -> Self {
    self.file_path = Some(file_path.into());
    self
  }

  pub fn memory_max_size(mut self, size: usize) -> Self {
    self.memory_max_size = Some(size);
    self
  }

  pub fn disk_max_size(mut self, size: usize) -> Self {
    self.disk_max_size = Some(size);
    self
  }

  pub fn build(self) -> GeoCacheConfig {
    let default = GeoCacheConfig::default();

    GeoCacheConfig {
      file_path: self.file_path.unwrap_or(default.file_path),
      memory_max_size: self.memory_max_size.unwrap_or(default.memory_max_size),
      disk_max_size: self.disk_max_size.unwrap_or(default.disk_max_size),
    }
  }
}

pub struct GeoCacheConfig {
  file_path: PathBuf,
  memory_max_size: usize,
  disk_max_size: usize,
}

impl Default for GeoCacheConfig {
  fn default() -> Self {
    Self {
      file_path: PathBuf::from("./geocache.db"),
      memory_max_size: 100 * 1024 * 1024, // 100MB,
      disk_max_size: 1024 * 1024 * 1024,  // 1GB,
    }
  }
}

impl GeoCacheConfig {
  pub fn builder() -> GeoCacheConfigBuilder {
    GeoCacheConfigBuilder::default()
  }
}

pub struct GeoCache {
  config: GeoCacheConfig,
  data: HashMap<CacheKey, Address>,
}

impl GeoCache {
  pub fn new(config: GeoCacheConfig) -> Self {
    Self {
      config,
      data: HashMap::new(),
    }
  }

  pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&self.config.file_path)?;
    let json: HashMap<CacheKeyRaw, Address> = serde_json::from_reader(file)?;

    println!("{:?}", json);

    Ok(())

    // let file = File::open(path)
  }

  /// Inserts a new key into `GeoCache` data.
  /// The key consists of `(latitude, longitude, language_code)`.
  pub fn insert(
    &mut self,
    (lat, lng, lang): (f64, f64, &str),
    address: Address,
  ) -> Result<(), Box<dyn Error>> {
    self
      .data
      .insert(CacheKey::try_new(lat, lng, lang)?, address);

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
  use std::path::PathBuf;

  use crate::{
    cache::{CacheKey, CacheKeyRaw, GeoCache, GeoCacheConfig, GeoCacheConfigBuilder},
    errors::{CountryCodeError, GeoCoordError},
  };

  fn create_example_geo_cache_config() -> GeoCacheConfig {
    GeoCacheConfigBuilder::default().build()
  }

  fn create_example_geo_cache() -> GeoCache {
    GeoCache::new(create_example_geo_cache_config())
  }

  #[test]
  fn geocache_config_builder() {
    let geocache_builder = GeoCacheConfig::builder()
      .file_path("./custom_path.db")
      .memory_max_size(555)
      .disk_max_size(333)
      .build();

    assert_eq!(geocache_builder.memory_max_size, 555);
    assert_eq!(geocache_builder.disk_max_size, 333);
    assert_eq!(
      geocache_builder.file_path,
      PathBuf::from("./custom_path.db")
    );
  }

  #[test]
  fn test_cache_key_try_new() {
    let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk");
    assert!(cache_key.is_ok());

    let cache_key_validate = CacheKey {
      lang: 29547,
      lat: 48164582,
      lng: 17184710,
    };

    assert_eq!(cache_key_validate, cache_key.unwrap());
  }

  #[test]
  fn test_cache_key_try_new_wrong_coordinations() {
    let cache_key = CacheKey::try_new(95.000033, 17.1847104, "sk");
    assert!(cache_key.is_err());

    let geo_coords_error = cache_key.unwrap_err();
    assert!(geo_coords_error.downcast_ref::<GeoCoordError>().is_some());
  }

  #[test]
  fn test_cache_key_try_new_wrong_country_code() {
    let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "skx");
    assert!(cache_key.is_err());

    let country_code_error = cache_key.unwrap_err();
    assert!(
      country_code_error
        .downcast_ref::<CountryCodeError>()
        .is_some()
    );
  }

  #[test]
  fn test_cache_get() {
    let mut geo_cache = create_example_geo_cache();

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
  fn test_cache_get_failed() {
    let mut geo_cache = create_example_geo_cache();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "sk"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(geo_cache.get((88.1645819, 17.1847104, "sk")).unwrap(), None)
  }

  #[test]
  fn convert_geo_key_to_geo_key_raw() {
    // let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk").unwrap();
    //
    // let cache_key_raw: CacheKeyRaw = cache_key.into();
    //
    // println!("raw {:?}", cache_key_raw);

    // assert_eq!(
    //   [
    //     101, 110, 59, 52, 56, 49, 54, 52, 53, 56, 49, 59, 49, 55, 49, 56, 52, 55, 49, 48
    //   ],
    //   cache_key_raw.0
    // )
  }
}
