use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder};

fn main() {
  let config = GeoCacheConfigBuilder::default().build();
  let mut geo_cache = GeoCache::<DequeStorage>::new(&config);

  geo_cache
    .insert(
      (48.1645819, 17.1847104, "sk"),
      "Bratislava, Slovakia".to_string(),
    )
    .expect("failed to insert address");

  geo_cache
    .insert((50.073658, 14.418540, "en"), "Prague, Czechia".to_string())
    .expect("failed to insert address");

  geo_cache
    .insert((52.520008, 13.404954, "de"), "Berlin, Germany".to_string())
    .expect("failed to insert address");

  assert_eq!(geo_cache.in_memory_record_count(), 3);

  let address = geo_cache
    .get((48.1645819, 17.1847104, "sk"))
    .expect("error while looking up address")
    .expect("address not found");
  println!("Found: {address}");

  // Cache miss — microdegree precision is required; approximate coords won't match
  let miss = geo_cache
    .get((48.1646, 17.1847, "sk"))
    .expect("error while looking up address");
  assert!(miss.is_none());
  println!("Approximate coordinates correctly produced a cache miss");
}
