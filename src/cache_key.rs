use std::{array::TryFromSliceError, ops::Deref};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
  errors::GeoCacheError,
  geo::{convert_coords_into_microdeg, convert_lang_to_u16, convert_u16_to_lang},
};

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
    let bytes = s.as_bytes();

    if bytes.len() != 20 {
      return Err(serde::de::Error::custom(
        GeoCacheError::CacheKeyRawInvalidLength { len: bytes.len() },
      ));
    }

    if !bytes
      .iter()
      .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
    {
      return Err(serde::de::Error::custom(
        GeoCacheError::CacheKeyRawInvalidCharacters { value: s },
      ));
    }

    let mut buf = [0u8; 20];
    buf.copy_from_slice(bytes);
    Ok(CacheKeyRaw(buf))
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
  /// [`GeoError::LatitudeOutOfRange`] if `lat` is outside `[-90, 90]`.
  /// [`GeoError::LongitudeOutOfRange`] if `lng` is outside `[-180, 180]`.
  /// [`GeoError::CountryCodeWrongLength`] if `lang` is not exactly 2 bytes.
  /// [`GeoError::InvalidCountryCode`] if `lang` contains non-alphabetic characters.
  pub fn try_new(lat: f64, lng: f64, lang: &str) -> Result<Self, GeoCacheError> {
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
  use crate::{cache_key::CacheKey, errors::GeoCacheError};

  #[test]
  fn test_cache_key_try_new_valid() {
    let key = CacheKey::try_new(48.1645819, 17.1847104, "sk");
    assert!(key.is_ok());
  }

  #[test]
  fn test_cache_key_try_new_boundary_latitude_min() {
    let key = CacheKey::try_new(-90.0, 17.1847104, "sk");
    assert!(key.is_ok());
  }

  #[test]
  fn test_cache_key_try_new_boundary_latitude_max() {
    let key = CacheKey::try_new(90.0, 17.1847104, "sk");
    assert!(key.is_ok());
  }

  #[test]
  fn test_cache_key_try_new_boundary_longitude_min() {
    let key = CacheKey::try_new(48.1645819, -180.0, "sk");
    assert!(key.is_ok());
  }

  #[test]
  fn test_cache_key_try_new_boundary_longitude_max() {
    let key = CacheKey::try_new(48.1645819, 180.0, "sk");
    assert!(key.is_ok());
  }

  #[test]
  fn test_cache_key_try_new_latitude_just_above_max() {
    let err = CacheKey::try_new(90.000001, 17.1847104, "sk").unwrap_err();
    assert!(matches!(err, GeoCacheError::LatitudeOutOfRange { value } if value == 90.000001));
  }

  #[test]
  fn test_cache_key_try_new_latitude_just_below_min() {
    let err = CacheKey::try_new(-90.000001, 17.1847104, "sk").unwrap_err();
    assert!(matches!(err, GeoCacheError::LatitudeOutOfRange { value } if value == -90.000001));
  }

  #[test]
  fn test_cache_key_try_new_longitude_just_above_max() {
    let err = CacheKey::try_new(48.1645819, 180.000001, "sk").unwrap_err();
    assert!(matches!(err, GeoCacheError::LongitudeOutOfRange { value } if value == 180.000001));
  }

  #[test]
  fn test_cache_key_try_new_longitude_just_below_min() {
    let err = CacheKey::try_new(48.1645819, -180.000001, "sk").unwrap_err();
    assert!(matches!(err, GeoCacheError::LongitudeOutOfRange { value } if value == -180.000001));
  }

  #[test]
  fn test_cache_key_try_new_lang_too_short() {
    let err = CacheKey::try_new(48.1645819, 17.1847104, "s").unwrap_err();
    assert!(matches!(
      err,
      GeoCacheError::CountryCodeWrongLength { len: 1 }
    ));
  }

  #[test]
  fn test_cache_key_try_new_lang_empty() {
    let err = CacheKey::try_new(48.1645819, 17.1847104, "").unwrap_err();
    assert!(matches!(
      err,
      GeoCacheError::CountryCodeWrongLength { len: 0 }
    ));
  }

  #[test]
  fn test_cache_key_try_new_lang_too_long() {
    let err = CacheKey::try_new(48.1645819, 17.1847104, "SVKK").unwrap_err();
    assert!(matches!(
      err,
      GeoCacheError::CountryCodeWrongLength { len: 4 }
    ));
  }

  #[test]
  fn test_cache_key_try_new_lang_numeric() {
    let err = CacheKey::try_new(48.1645819, 17.1847104, "42").unwrap_err();
    assert!(matches!(err, GeoCacheError::InvalidCountryCode { ref code } if code == "42"));
  }

  #[test]
  fn test_cache_key_try_new_lang_special_chars() {
    let err = CacheKey::try_new(48.1645819, 17.1847104, "s!").unwrap_err();
    assert!(matches!(err, GeoCacheError::InvalidCountryCode { ref code } if code == "s!"));
  }

  #[test]
  fn test_cache_key_try_new_lat_error_before_lng() {
    let err = CacheKey::try_new(95.0, 200.0, "sk").unwrap_err();
    assert!(matches!(err, GeoCacheError::LatitudeOutOfRange { .. }));
  }

  #[test]
  fn test_cache_key_try_new_coord_error_before_lang() {
    let err = CacheKey::try_new(95.0, 17.1847104, "SVK").unwrap_err();
    assert!(matches!(err, GeoCacheError::LatitudeOutOfRange { .. }));
  }
}
