use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder, StorageFlushStrategy};

fn main() {
  let config = GeoCacheConfigBuilder::default()
    .storage_flush_strategy(StorageFlushStrategy::Immediately)
    .storage_file_path("./geoverse_db.bin")
    .build();

  let mut geo_cache = GeoCache::<DequeStorage>::new(config);

  geo_cache
    .insert(
      (48.1645819, 17.1847104, "sk"),
      "Bratislava, Slovakia".to_string(),
    )
    .unwrap();

  let address = geo_cache.get((48.1645819, 17.1847104, "sk")).unwrap();

  println!("{:?}", address);
}
