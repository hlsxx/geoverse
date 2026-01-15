use std::error::Error;

use crate::{
  errors::CountryCodeError, errors::GeoCoordError, throw_country_code_error, throw_geo_coord_error,
};

/// Multiplier to convert degrees to microdegrees (1 degree = 1,000,000 microdegrees)
const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;

/// Converts latitude and longitude from decimal degrees to microdegrees.
///
/// Microdegrees are degrees multiplied by 1,000,000, allowing geographic
/// coordinates to be stored as integers to avoid floating-point precision issues.
///
/// # Arguments
/// * `lat` - Latitude in decimal degrees, must be in range [-90, 90]
/// * `lng` - Longitude in decimal degrees, must be in range [-180, 180]
///
/// # Returns
/// * `Ok((i32, i32))` - Tuple of (latitude_microdeg, longitude_microdeg)
/// * `Err(GeoCoordError)` - If coordinates are out of valid range
///
/// # Examples
/// ```
/// // Bratislava coordinates: 48°08'38"NN, 17°06'35"E
/// let (lat, lng) = geoverse::convert_coords_into_microdeg(48.0838, 17.0635).unwrap();
/// assert_eq!(lat, 48083800);
/// assert_eq!(lng, 17063500);
/// ```
pub fn convert_coords_into_microdeg(lat: f64, lng: f64) -> Result<(i32, i32), GeoCoordError> {
  if lat > 90.0 || lat < -90.0 {
    throw_geo_coord_error!("Latitude is out of valid range [-90, 90]");
  }
  if lng > 180.0 || lng < -180.0 {
    throw_geo_coord_error!("Longitude is out of valid range [-180, 180]");
  }
  let lat = (lat * MICRODEGREES_PER_DEGREE).round() as i32;
  let lng = (lng * MICRODEGREES_PER_DEGREE).round() as i32;
  Ok((lat, lng))
}

/// Converts a 2-character language code into a u16 by packing the bytes.
///
/// Each character is converted to its ASCII byte value, then the two bytes
/// are combined into a u16 using big-endian byte order (first character in
/// the high byte, second character in the low byte).
///
/// # Arguments
/// * `lang` - A 2-character language code (e.g., "sk", "en", "de")
///
/// # Returns
/// * `Ok(u16)` - The packed u16 representation
/// * `Err(CountryCodeError)` - If the input is not exactly 2 characters
///
/// # Examples
/// ```
/// // "sk" -> bytes [115, 107] -> u16: 29547
/// // 's' (115 = 0x73) in high byte, 'k' (107 = 0x6B) in low byte
/// let code = geoverse::convert_lang_to_u16("sk").unwrap();
/// assert_eq!(code, 0x736B); // or 29547 in decimal
/// ```
pub fn convert_lang_to_u16(lang: &str) -> Result<u16, Box<dyn Error>> {
  let lang_bytes = lang.as_bytes();
  if lang.len() == 2 {
    Ok(u16::from_be_bytes([lang_bytes[0], lang_bytes[1]]))
  } else {
    throw_country_code_error!(
      "Invalid country code (must be exactly 2 characters [ISO 3166-1 alpha-2])"
    )
  }
}

/// Converts a u16 into a 2-character language code.
///
/// Unpacks the u16 by extracting the high and low bytes using big-endian
/// byte order, then converts them back to ASCII characters.
///
/// # Arguments
/// * `code` - The packed u16 representation of a language code
///
/// # Returns
/// * `Ok(String)` - The 2-character language code
/// * `Err(GeoCoordError)` - If the bytes don't form valid UTF-8
///
/// # Examples
/// ```
/// let code = geoverse::convert_lang_to_u16("sk").unwrap();
/// let lang = geoverse::convert_u16_to_lang(code).unwrap();
/// assert_eq!(lang, "sk");
/// ```
pub fn convert_u16_to_lang(code: u16) -> Result<String, CountryCodeError> {
  let bytes = code.to_be_bytes();
  String::from_utf8(bytes.to_vec()).map_err(|_| CountryCodeError {
    message: "Invalid UTF-8 in language code".to_string(),
  })
}
