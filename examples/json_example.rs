use geoverse::cache::{GeoCache, GeoCacheConfigBuilder};

fn main() {
  let config = GeoCacheConfigBuilder::default()
    .file_path("./geoverse.json")
    .build();

  let geo_cache = GeoCache::new(config).init().unwrap();
}
