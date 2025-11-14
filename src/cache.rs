use std::{collections::HashMap, error::Error};

use crate::{
  errors::CountryCodeError, geo::convert_coords_into_microdeg, throw_country_code_error,
};

type Address = String;

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
  /// use geovomit::cache::CacheKey;
  /// let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk");
  /// ```
  ///
  /// # Errors
  ///
  /// * Provided lang and lat must fit in specific range
  /// * Provided language must be correct country code size of 2
  pub fn try_new(lat: f64, lng: f64, lang: &str) -> Result<Self, Box<dyn Error>> {
    let lang_bytes = lang.as_bytes();

    let (lat, lng) = convert_coords_into_microdeg(lat, lng)?;

    let lang_as_u16 = if lang.len() == 2 {
      u16::from_be_bytes([lang_bytes[0], lang_bytes[1]])
    } else {
      throw_country_code_error!();
    };

    Ok(Self {
      lat,
      lng,
      lang: lang_as_u16,
    })
  }
}

pub struct GeoCache {
  data: HashMap<CacheKey, Address>,
}

impl GeoCache {
  pub fn new() -> Self {
    Self {
      data: HashMap::new(),
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
  use crate::{
    cache::{CacheKey, GeoCache},
    errors::{CountryCodeError, GeoCoordError},
  };

  #[test]
  fn test_cache_key_try_new() {
    let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk");
    assert!(cache_key.is_ok());
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
    let mut geo_cache = GeoCache::new();

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
    let mut geo_cache = GeoCache::new();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "sk"),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    assert_eq!(geo_cache.get((88.1645819, 17.1847104, "sk")).unwrap(), None)
  }
}
