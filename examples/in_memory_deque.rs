use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder};

fn main() {
  let config = GeoCacheConfigBuilder::default().build();
  let mut geo_cache = GeoCache::<DequeStorage>::new(config);

  geo_cache
    .insert(
      (48.1645819, 17.1847104, "sk"),
      "Bratislava, Slovakia".to_string(),
    )
    .expect("failed to insert address");

  let address = geo_cache
    .get((48.1645819, 17.1847104, "sk"))
    .expect("error while loading address")
    .expect("address not found");

  println!("{address}");
}
