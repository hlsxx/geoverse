use std::path::PathBuf;

pub struct GeoCacheConfig {
  pub file_path: PathBuf,
  pub memory_max_size: usize,
  pub disk_max_size: usize,
}

impl Default for GeoCacheConfig {
  fn default() -> Self {
    Self {
      file_path: PathBuf::from("./geocache.db"),
      memory_max_size: 100 * 1024 * 1024, // 100MB,
      disk_max_size: 1024 * 1024 * 1024,  // 1GB,
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
  file_path: Option<PathBuf>,
  memory_max_size: Option<usize>,
  disk_max_size: Option<usize>,
}

impl GeoCacheConfigBuilder {
  pub fn file_path(mut self, file_path: impl Into<PathBuf>) -> Self {
    self.file_path = Some(file_path.into());
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
      file_path: self.file_path.unwrap_or(default.file_path),
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
      .file_path("./custom_path.db")
      .memory_max_size(555)
      .disk_max_size(333)
      .build();

    assert_eq!(geocache_builder.memory_max_size, 555);
    assert_eq!(geocache_builder.disk_max_size, 333);
    assert_eq!(
      geocache_builder.file_path,
      PathBuf::from("./custom_path.db")
    );
  }
}
