use crate::{errors::GeoCoordError, throw_geo_coord_error};

const MILLION: f64 = 1_000_000.0;

pub fn convert_coords_into_microdeg(lat: f64, lng: f64) -> Result<(i32, i32), GeoCoordError> {
  if lat > 90.0 || lat < -90.0 {
    throw_geo_coord_error!("Latitude is out of valid range [-90, 90]");
  }

  if lng > 180.0 || lng < -180.0 {
    throw_geo_coord_error!("Longitude is out of valid range [-180, 180]");
  }

  let lat = (lat * MILLION).round() as i32;
  let lng = (lng * MILLION).round() as i32;

  println!("lat {lat} lang {lng}");

  Ok((lat, lng))
}
