pub mod deque;
pub mod lru;

use std::{
  error::Error,
  fs::File,
  io::{self, Read, Seek, Write},
  path::Path,
};

use crate::cache_key::CacheKey;

// TODO: Convert to the new type
// Check Address lenght (255 max)
pub type Address = String;

/// Defines how often to flush data to the persistence disk.
#[derive(PartialEq, Eq, Default)]
pub enum StorageFlushStrategy {
  /// Keep data in-memory only, never flush to disk.
  #[default]
  Never,
  /// Flush to disk after every write operation.
  Immediately,
  /// Flush to disk after a specific number of records have been written.
  RecordCount(usize),
}

/// Defines the interface for persistence storage mechanism used by cache implementions.
///
/// Implement this trait to provide a custom storage strategy, e.g., Deque or LRU.
pub trait StorageStrategy {
  // Defines how many items will dropped at delete call
  const ON_DELETE_ITEMS_COUNT: usize;

  /// Inserts a `cache_key` with its associated `address` into the storage.
  ///
  /// Returns an error if the insertion fails.
  fn insert(&mut self, cache_key: CacheKey, address: Address) -> Result<(), Box<dyn Error>>;

  /// Retrieves the `Address` associated with the given `cache_key`, if it exists.
  fn get(&self, cache_key: &CacheKey) -> Option<&Address>;

  /// Serializes the storage contents into raw bytes.
  fn as_bytes(&self) -> Vec<u8>;

  /// Reads data from the given `storage` file into a memory.
  fn read(&mut self, storage: &mut Storage) -> io::Result<()>;

  /// Flushes the current storage state into the given `storage` file.
  fn flush(&self, storage: &mut Storage) -> io::Result<()>;

  /// Deletes a record with specific conditions
  fn delete(&mut self, storage: &mut Storage) -> io::Result<()>;
}

pub struct Storage {
  /// The underlying file used for persistent storage
  file: File,
  /// Indicates whether data has been modified since the last flush to disk
  is_dirty: bool,
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

    Ok(Self {
      file,
      is_dirty: false,
    })
  }

  pub fn len(&self) -> io::Result<u64> {
    Ok(self.file.metadata()?.len())
  }

  /// Reads all the data from the `storage` file.
  ///
  /// # Errors
  /// Returns an error when reading fails.
  pub fn read(&mut self) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    self.file.seek(io::SeekFrom::Start(0))?;
    self.file.read_to_end(&mut buf)?;
    Ok(buf)
  }

  /// Writes a bytes into the `storage` file.
  ///
  /// # Errors
  /// Returns and error if writing fails.
  pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
    self.is_dirty = true;
    self.file.write_all(bytes)
  }

  /// Truncates and writes a bytes into the `storage` file.
  ///
  /// # Errors
  /// Returns and error if truncate fails.
  /// Returns and error if writing fails.
  pub fn truncate_and_write(&mut self, bytes: &[u8]) -> io::Result<()> {
    self.file.set_len(0)?;
    self.file.seek(io::SeekFrom::Start(0))?;
    self.write(bytes)
  }

  /// Flushes data to persistent storage.
  ///
  /// Only performs the flush if data has been modified since the last sync.
  pub fn sync(&mut self) -> io::Result<()> {
    if self.is_dirty {
      self.file.flush()?;
      self.is_dirty = false;
    }

    Ok(())
  }
}

impl Drop for Storage {
  fn drop(&mut self) {
    let _ = self.sync();
  }
}
