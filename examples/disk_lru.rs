use geoverse::{GeoCache, GeoCacheConfigBuilder, LruStorage, StorageFlushStrategy};

fn main() {
  let path = "geoverse_lru.bin";

  // First session: insert and persist to disk
  {
    let config = GeoCacheConfigBuilder::default()
      .storage_file_path(path)
      .storage_flush_strategy(StorageFlushStrategy::Immediately)
      .build();

    let mut geo_cache = GeoCache::<LruStorage>::new(&config);
    geo_cache.init().unwrap();

    geo_cache
      .insert(
        (48.1645819, 17.1847104, "sk"),
        "Bratislava, Slovakia".to_string(),
      )
      .expect("failed to insert address");

    geo_cache
      .insert((50.073658, 14.418540, "en"), "Prague, Czechia".to_string())
      .expect("failed to insert address");

    println!(
      "Session 1 — records in cache: {}",
      geo_cache.in_memory_record_count()
    );
  }

  // Second session: reload LRU data from disk and verify
  {
    let config = GeoCacheConfigBuilder::default()
      .storage_file_path(path)
      .build();

    let mut geo_cache = GeoCache::<LruStorage>::new(&config);
    geo_cache.init().unwrap();

    let address = geo_cache
      .get((48.1645819, 17.1847104, "sk"))
      .expect("error while loading address")
      .expect("address not found");

    println!("Session 2 — reloaded: {address}");
    assert_eq!(address, "Bratislava, Slovakia");
    assert_eq!(geo_cache.in_memory_record_count(), 2);
  }

  std::fs::remove_file(path).ok();
}
