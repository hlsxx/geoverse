use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeoCacheError {
  #[error("latitude {value} is out of range [-90, 90]")]
  LatitudeOutOfRange { value: f64 },

  #[error("longitude {value} is out of range [-180, 180]")]
  LongitudeOutOfRange { value: f64 },

  #[error("invalid coordinate format: {input:?}")]
  InvalidFormat { input: String },

  #[error("invalid country code: {code:?} (must be ISO 3166-1 alpha-2)")]
  InvalidCountryCode { code: String },

  #[error("country code has wrong lenght: expected 2 chars, got {len}")]
  CountryCodeWrongLength { len: usize },

  #[error("invalid cache key length: expected 20 bytes, got {len}")]
  CacheKeyRawInvalidLength { len: usize },

  #[error("invalid characters in cache key: {value:?} (only ASCII alphanumeric and '-' allowed)")]
  CacheKeyRawInvalidCharacters { value: String },
}
