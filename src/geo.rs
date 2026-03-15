use crate::errors::GeoCacheError;

/// Multiplier to convert degrees to microdegrees (1 degree = 1,000,000 microdegrees)
const MICRODEGREES_PER_DEGREE: f64 = 1_000_000.0;

/// Converts latitude and longitude from decimal degrees to microdegrees.
///
/// Microdegrees are degrees multiplied by 1,000,000, allowing geographic
/// coordinates to be stored as integers to avoid floating-point precision issues.
///
/// # Arguments
/// * `lat` - Latitude in decimal degrees, must be in range `[-90, 90]`
/// * `lng` - Longitude in decimal degrees, must be in range `[-180, 180]`
///
/// # Returns
/// A tuple of `(latitude_microdeg, longitude_microdeg)` as `(i32, i32)`.
///
/// # Errors
/// Returns [`GeoCacheError::LatitudeOutOfRange`] if `lat` is outside `[-90, 90]`.
/// Returns [`GeoCacheError::LongitudeOutOfRange`] if `lng` is outside `[-180, 180]`.
///
/// # Examples
/// ```
/// // Bratislava coordinates: 48°08'38"N, 17°06'35"E
/// let (lat, lng) = geoverse::convert_coords_into_microdeg(48.0838, 17.0635).unwrap();
/// assert_eq!(lat, 48083800);
/// assert_eq!(lng, 17063500);
/// ```
pub fn convert_coords_into_microdeg(lat: f64, lng: f64) -> Result<(i32, i32), GeoCacheError> {
  if !(-90.0..=90.0).contains(&lat) {
    return Err(GeoCacheError::LatitudeOutOfRange { value: lat });
  }
  if !(-180.0..=180.0).contains(&lng) {
    return Err(GeoCacheError::LongitudeOutOfRange { value: lng });
  }
  let lat = (lat * MICRODEGREES_PER_DEGREE).round() as i32;
  let lng = (lng * MICRODEGREES_PER_DEGREE).round() as i32;
  Ok((lat, lng))
}

/// Converts a 2-character language code into a `u16` by packing the bytes.
///
/// Each character is converted to its ASCII byte value, then the two bytes
/// are combined into a `u16` using big-endian byte order (first character in
/// the high byte, second character in the low byte).
///
/// # Arguments
/// * `lang` - A 2-character ASCII alphabetic language code (e.g., `"sk"`, `"en"`, `"de"`)
///
/// # Returns
/// The packed `u16` representation of the language code.
///
/// # Errors
/// Returns [`GeoCacheError::CountryCodeWrongLength`] if `lang` is not exactly 2 bytes.
/// Returns [`GeoCacheError::InvalidCountryCode`] if either byte is not an ASCII letter.
///
/// # Examples
/// ```
/// // "sk" -> bytes [0x73, 0x6B] -> u16: 0x736B (29547)
/// let code = geoverse::convert_lang_to_u16("sk").unwrap();
/// assert_eq!(code, 0x736B);
/// ```
pub fn convert_lang_to_u16(lang: &str) -> Result<u16, GeoCacheError> {
  let bytes = lang.as_bytes();

  match bytes {
    [b1, b2] if b1.is_ascii_alphabetic() && b2.is_ascii_alphabetic() => {
      Ok(u16::from_be_bytes([*b1, *b2]))
    }
    [_, _] => Err(GeoCacheError::InvalidCountryCode {
      code: lang.to_string(),
    }),
    _ => Err(GeoCacheError::CountryCodeWrongLength { len: bytes.len() }),
  }
}

/// Converts a `u16` back into a 2-character language code.
///
/// This is the inverse of [`convert_lang_to_u16`]. Unpacks the `u16` by
/// extracting the high and low bytes using big-endian byte order, then
/// converts them back to ASCII characters.
///
/// # Arguments
/// * `code` - A `u16` produced by [`convert_lang_to_u16`]
///
/// # Returns
/// The 2-character language code as a `String`.
///
/// # Errors
/// Returns [`GeoCacheError::InvalidCountryCode`] if either byte is not an ASCII letter.
/// The error includes the hex representation of the invalid code (e.g., `"0x0041"`).
///
/// # Examples
/// ```
/// let code = geoverse::convert_lang_to_u16("sk").unwrap();
/// let lang = geoverse::convert_u16_to_lang(code).unwrap();
/// assert_eq!(lang, "sk");
/// ```
pub fn convert_u16_to_lang(code: u16) -> Result<String, GeoCacheError> {
  let [b1, b2] = code.to_be_bytes();

  if !b1.is_ascii_alphabetic() || !b2.is_ascii_alphabetic() {
    return Err(GeoCacheError::InvalidCountryCode {
      code: format!("0x{code:04X}"),
    });
  }

  Ok(format!("{}{}", b1 as char, b2 as char))
}
