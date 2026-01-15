use std::path::PathBuf;

use crate::storage::StorageFlushStrategy;

pub struct GeoCacheConfig {
  /// Path to a persistance disk
  pub storage_file_path: Option<PathBuf>,

  /// Limit memory max usage (MB)
  pub memory_max_size: usize,

  /// Limit persistance disk usage (MB)
  pub disk_max_size: usize,

  /// Storage flush strategy
  pub storage_flush_strategy: StorageFlushStrategy,
}

impl Default for GeoCacheConfig {
  fn default() -> Self {
    Self {
      storage_file_path: None,
      memory_max_size: 100 * 1024 * 1024, // 100MB,
      disk_max_size: 1024 * 1024 * 1024,  // 1GB,
      storage_flush_strategy: StorageFlushStrategy::default(),
    }
  }
}

impl GeoCacheConfig {
  pub fn builder() -> GeoCacheConfigBuilder {
    GeoCacheConfigBuilder::default()
  }
}

#[derive(Default)]
pub struct GeoCacheConfigBuilder {
  storage_file_path: Option<PathBuf>,

  memory_max_size: Option<usize>,

  disk_max_size: Option<usize>,

  storage_flush_strategy: Option<StorageFlushStrategy>,
}

impl GeoCacheConfigBuilder {
  pub fn storage_file_path(mut self, file_path: impl Into<PathBuf>) -> Self {
    self.storage_file_path = Some(file_path.into());
    self
  }

  pub fn storage_flush_strategy(mut self, flush_strategy: StorageFlushStrategy) -> Self {
    self.storage_flush_strategy = Some(flush_strategy);
    self
  }

  pub fn memory_max_size(mut self, size: usize) -> Self {
    self.memory_max_size = Some(size);
    self
  }

  pub fn disk_max_size(mut self, size: usize) -> Self {
    self.disk_max_size = Some(size);
    self
  }

  pub fn build(self) -> GeoCacheConfig {
    let default = GeoCacheConfig::default();

    GeoCacheConfig {
      storage_file_path: self.storage_file_path,
      storage_flush_strategy: self
        .storage_flush_strategy
        .unwrap_or(default.storage_flush_strategy),
      memory_max_size: self.memory_max_size.unwrap_or(default.memory_max_size),
      disk_max_size: self.disk_max_size.unwrap_or(default.disk_max_size),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::cache_config::GeoCacheConfig;

  #[test]
  fn geocache_config_builder() {
    let geocache_builder = GeoCacheConfig::builder()
      .storage_file_path("./geoverse_db.bin")
      .memory_max_size(555)
      .disk_max_size(333)
      .build();

    assert_eq!(geocache_builder.memory_max_size, 555);
    assert_eq!(geocache_builder.disk_max_size, 333);
    assert_eq!(
      geocache_builder.storage_file_path,
      Some(PathBuf::from("./geoverse_db.bin"))
    );
  }
}
