use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct GeoCoordError {
  pub message: String,
}

#[derive(Debug)]
pub struct CountryCodeError;

impl Display for GeoCoordError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl Display for CountryCodeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Invalid country code: must be exactly 2 characters")
  }
}

impl Error for GeoCoordError {}
impl Error for CountryCodeError {}

#[macro_export]
macro_rules! throw_geo_coord_error {
  ($x:expr) => {
    return Err(GeoCoordError {
      message: $x.to_string(),
    })
  };
}

#[macro_export]
macro_rules! throw_country_code_error {
  () => {
    return Err(Box::new(CountryCodeError) as Box<dyn Error>)
  };
}
