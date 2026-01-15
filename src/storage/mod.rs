pub mod deque;
pub mod lru;

use std::{error::Error, fs::File, io, path::Path};

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::{CacheKey, CacheKeyRaw},
};

pub type Address = String;

pub struct Storage {
  file: File,
}

pub trait StorageStrategy {
  fn insert(&mut self, cache_key: CacheKey, address: Address) -> Result<(), Box<dyn Error>>;
  fn get(&self, cache_key: &CacheKey) -> Option<&Address>;
  fn flush(&self) -> io::Result<()>;
}

impl Storage {
  /// Tries to open or create a persistent `storage` file.
  ///
  /// # Errors
  /// Returns an io error if the file cannot be opened or created.
  pub fn try_new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
    let file = File::options()
      .read(true)
      .write(true)
      .create(true)
      .open(path)?;

    Ok(Self { file })
  }

  /// Reads all the data from the `storage` file.
  ///
  /// # Errors
  /// Returns an error when reading fails.
  pub fn read(&self) -> io::Result<Vec<u8>> {
    !unimplemented!()
  }

  /// Writes a cache key entry to the `storage` file.
  ///
  /// # Errors
  /// Returns and error if writing fails.
  pub fn write(&mut self, cache_key_raw: &CacheKeyRaw) -> io::Result<()> {
    !unimplemented!()
  }
}
