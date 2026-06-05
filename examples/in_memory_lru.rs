use geoverse::{GeoCache, GeoCacheConfigBuilder, LruStorage};

fn main() {
  let config = GeoCacheConfigBuilder::default().build();
  let mut geo_cache = GeoCache::<LruStorage>::new(&config);

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

  // LRU promotes accessed entries: repeatedly reading Bratislava keeps it hot
  for _ in 0..5 {
    let _ = geo_cache
      .get((48.1645819, 17.1847104, "sk"))
      .expect("error while looking up address");
  }

  let address = geo_cache
    .get((48.1645819, 17.1847104, "sk"))
    .expect("error while looking up address")
    .expect("address not found");
  println!("Found (LRU-hot): {address}");
}
