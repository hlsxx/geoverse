pub mod deque;
pub mod lru;

use std::{
  error::Error,
  fs::File,
  io::{self, Write},
  path::Path,
};

use crate::{
  cache_config::GeoCacheConfig,
  cache_key::{CacheKey, CacheKeyRaw},
};

pub type Address = String;

/// Storage flush strategy
///
/// Defines how often flush to the presistance disk
#[derive(PartialEq, Eq)]
pub enum StorageFlushStrategy {
  /// Use just in-memory data
  Never,

  /// Flush after every write operation
  Immediately,

  /// Flush after specific record count
  RecordCount(usize),
}

impl Default for StorageFlushStrategy {
  fn default() -> Self {
    Self::Never
    // // After new 30 records added flush to the persistance disk
    // Self::RecordCount(30)
  }
}

/// Storage strategy for caching operations and persistance operations
///
/// Defines the interface for persistent storage mechanisms used by cache implementations.
/// e.g.: Deque, LRU strategy
pub trait StorageStrategy {
  fn insert(&mut self, cache_key: CacheKey, address: Address) -> Result<(), Box<dyn Error>>;

  fn get(&self, cache_key: &CacheKey) -> Option<&Address>;

  fn as_bytes(&self) -> Vec<u8>;

  /// Flush into the `storage file`
  fn flush(&self, storage: &mut Storage) -> io::Result<()>;
}

pub struct Storage {
  file: File,
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

  /// Writes a bytes into the `storage` file.
  ///
  /// # Errors
  /// Returns and error if writing fails.
  pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
    self.file.write_all(&bytes)
  }
}
