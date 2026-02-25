use std::{array::TryFromSliceError, error::Error, ops::Deref};

use serde::{Deserialize, Deserializer, Serialize};

use crate::geo::{convert_coords_into_microdeg, convert_lang_to_u16, convert_u16_to_lang};

/// Raw byte representation of a cache key.
///
/// Format: `[lang_code (2 bytes)][';' (1 byte)][lat_str (8 bytes)][';' (1 byte)][lng_str (8 bytes)]`
///
/// The cache key encodes a language code and coordinates as ASCII bytes for efficient
/// storage and hashing. Coordinates are stored as fixed-width 8-byte ASCII strings
/// representing microdegrees with leading zeros if necessary.
///
/// # Layout (20 bytes total)
/// - Bytes 0-1: Language code (e.g., "sk" = [115, 107])
/// - Byte 2: Semicolon separator (';' = 59)
/// - Bytes 3-10: Latitude in microdegrees as 8 ASCII digits
/// - Byte 11: Semicolon separator (';' = 59)
/// - Bytes 12-19: Longitude in microdegrees as 8 ASCII digits
///
/// # Example
/// ```ignore
/// // "sk;48164582;17184710"
/// // sk: language code
/// // 48164582: latitude 48.164582° in microdegrees
/// // 17184710: longitude 17.184710° in microdegrees (Bratislava coordinates)
/// let key = crate::cache_key::CacheKeyRaw([
///   115, 107, 59,  // "sk;"
///   52, 56, 49, 54, 52, 53, 56, 50, 59,  // "48164582;"
///   49, 55, 49, 56, 52, 55, 49, 48       // "17184710"
/// ]);
/// ```
#[derive(Debug, Serialize, Hash, Eq, PartialEq)]
pub struct CacheKeyRaw(pub [u8; 20]);

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

impl Deref for CacheKeyRaw {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
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
  /// ```ignore
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

impl TryFrom<&[u8]> for CacheKeyRaw {
  type Error = TryFromSliceError;

  fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
    Ok(CacheKeyRaw(value.try_into()?))
  }
}

impl From<CacheKeyRaw> for CacheKey {
  fn from(value: CacheKeyRaw) -> Self {
    let lang = std::str::from_utf8(&value[0..2]).unwrap();
    let lat: i32 = std::str::from_utf8(&value[3..11]).unwrap().parse().unwrap();
    let lng: i32 = std::str::from_utf8(&value[12..20])
      .unwrap()
      .parse()
      .unwrap();

    CacheKey {
      lat,
      lng,
      lang: convert_lang_to_u16(lang).unwrap(),
    }
  }
}

impl From<CacheKey> for CacheKeyRaw {
  fn from(value: CacheKey) -> CacheKeyRaw {
    let mut bytes = [0u8; 20];
    let lang = convert_u16_to_lang(value.lang).unwrap();
    let s = format!("{};{:08};{:08}", lang, value.lat, value.lng);
    bytes.copy_from_slice(s.as_bytes());
    CacheKeyRaw(bytes)
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    cache_key::{CacheKey, CacheKeyRaw},
    errors::{CountryCodeError, GeoCoordError},
  };

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
  fn convert_geo_key_to_geo_key_raw() {
    let cache_key = CacheKey::try_new(48.1645819, 17.1847104, "sk").unwrap();
    let cache_key_raw: CacheKeyRaw = cache_key.into();

    assert_eq!(
      [
        115, 107, 59, 52, 56, 49, 54, 52, 53, 56, 50, 59, 49, 55, 49, 56, 52, 55, 49, 48
      ],
      cache_key_raw.0
    )
  }
}
