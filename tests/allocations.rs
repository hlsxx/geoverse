#![cfg(feature = "testing")]

use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder};

// Used in storage strategy internal data structures
const CAPACITY_SIZE: usize = 100;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn create_example_deque_geo_cache() -> GeoCache<DequeStorage> {
  GeoCache::with_capacity(GeoCacheConfigBuilder::default().build(), CAPACITY_SIZE)
}

#[test]
fn insert_should_not_over_allocate() {
  let mut cache = create_example_deque_geo_cache();

  let _profiler = dhat::Profiler::builder().testing().build();

  cache
    .insert((48.1645819, 17.1847104, "sk"), "Bratislava".to_string())
    .unwrap();

  let stats = dhat::HeapStats::get();
  dhat::assert_eq!(stats.total_blocks, 1);
}

#[test]
fn get_should_allocate_nothing() {
  let mut cache = create_example_deque_geo_cache();

  cache
    .insert((48.1645819, 17.1847104, "sk"), "Bratislava".to_string())
    .unwrap();

  let _profiler = dhat::Profiler::builder().testing().build();

  cache.get((48.1645819, 17.1847104, "sk")).unwrap();

  let stats = dhat::HeapStats::get();
  dhat::assert_eq!(stats.total_blocks, 0);
}
