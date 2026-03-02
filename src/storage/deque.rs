use std::collections::{HashMap, VecDeque};

use crate::{
  cache_key::{CacheKey, CacheKeyRaw},
  storage::{Address, Storage, StorageStrategy},
};

/// DequeStorage is a simple persistence storage technique
/// which consits of a `data` field and a `cache_keys` field.
///
/// Field `cache_keys` is essential for rememberering the insertion
/// order of each `CacheKey` - the oldest inserted key sits at
/// the back of the `VecDeque`.
#[derive(Default)]
pub struct DequeStorage {
  /// Maps each `CacheKey` to its corresponding `Address` in memory
  data: HashMap<CacheKey, Address>,
  /// Tracks insertion order of keys (oldest at the back)
  cache_keys: VecDeque<CacheKey>,
  /// Current total memory usage in bytes
  memory_size: usize,
}

impl StorageStrategy for DequeStorage {
  const ON_DELETE_ITEMS_COUNT: usize = 10;

  fn insert(
    &mut self,
    cache_key: CacheKey,
    address: Address,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.increment_size(address.len());
    self.insert_key(cache_key.clone());
    self.data.insert(cache_key, address);
    Ok(())
  }

  fn get(&self, cache_key: &CacheKey) -> Option<&Address> {
    self.data.get(cache_key)
  }

  fn get_in_memory_size(&self) -> usize {
    self.memory_size
  }

  fn as_bytes(&self) -> Vec<u8> {
    self
      .cache_keys
      .iter()
      .flat_map(|cache_key| {
        let address = self.data.get(cache_key).unwrap();
        DeqeueStorageItem::from_cache_key(cache_key, address).to_bytes()
      })
      .collect::<Vec<u8>>()
  }

  fn read(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    let bytes = storage.read()?;
    let mut pos = 0;

    // Get storage item len - address len
    const STORAGE_ITEM_LEN: usize = DeqeueStorageItem::len() - 1;

    while pos + STORAGE_ITEM_LEN <= bytes.len() {
      let key_bytes: [u8; STORAGE_ITEM_LEN] =
        bytes[pos..pos + STORAGE_ITEM_LEN].try_into().unwrap();
      let cache_key_raw = CacheKeyRaw(key_bytes);
      pos += STORAGE_ITEM_LEN;

      let addr_len = bytes[pos] as usize;
      pos += 1;

      let address = String::from_utf8_lossy(&bytes[pos..pos + addr_len]);
      pos += addr_len;

      let cache_key: CacheKey = cache_key_raw.into();

      self.data.insert(cache_key.clone(), address.to_string());
      self.cache_keys.push_front(cache_key);
      self.increment_size(address.len());
    }

    Ok(())
  }

  fn flush(&self, storage: &mut Storage) -> std::io::Result<()> {
    storage.truncate_and_write(&self.as_bytes())
  }

  fn evict_if_needed(&mut self, storage: &mut Storage, address_len: usize) -> std::io::Result<()> {
    if DeqeueStorageItem::len() + address_len > self.memory_size {
      self.evict(storage)?;
    }

    Ok(())
  }

  fn evict(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    self.cache_keys.truncate(
      self
        .cache_keys
        .len()
        .saturating_sub(Self::ON_DELETE_ITEMS_COUNT),
    );

    self.data.retain(|key, _| self.cache_keys.contains(key));

    self.flush(storage)?;

    // Size equals persistance disk size
    self.memory_size = storage.len()? as usize;

    Ok(())
  }
}

impl DequeStorage {
  fn insert_key(&mut self, cache_key: CacheKey) {
    self.cache_keys.push_front(cache_key.clone());
  }

  /// Increments in a memory size or persistance disk size
  fn increment_size(&mut self, address_len: usize) {
    self.memory_size += DeqeueStorageItem::len() + address_len;
  }

  /// Returns a first record
  #[allow(unused)]
  fn first(&self) -> Option<(&CacheKey, &Address)> {
    if let Some(cache_key) = self.cache_keys.front()
      && let Some(address) = self.data.get(cache_key)
    {
      return Some((cache_key, address));
    }

    None
  }

  /// Returns a last record
  #[allow(unused)]
  fn last(&self) -> Option<(&CacheKey, &Address)> {
    if let Some(cache_key) = self.cache_keys.back()
      && let Some(address) = self.data.get(cache_key)
    {
      return Some((cache_key, address));
    }

    None
  }
}

pub(crate) struct DeqeueStorageItem {
  // Raw cache key
  cache_key_raw: CacheKeyRaw,

  /// Address string length (limited to 255 length)
  address_len: u8,

  /// Geocoded address string
  address: Address,
}

impl DeqeueStorageItem {
  pub fn from_cache_key(cache_key: &CacheKey, address: &Address) -> Self {
    Self {
      cache_key_raw: cache_key.clone().into(),
      address_len: address.len() as u8,
      address: address.clone(),
    }
  }

  pub fn to_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&self.cache_key_raw);
    bytes.extend_from_slice(&self.address_len.to_be_bytes());
    bytes.extend_from_slice(self.address.as_bytes());
    bytes
  }

  #[allow(unused)]
  /// Size in a binary file.
  /// 2 (lang) + 2 * 8 (2 * lat/lng) + 2 (2 * ;) + 1 (size of the address [limited to 255])
  pub const fn len() -> usize {
    const LANG_SIZE: usize = 2;
    const COORD_SIZE: usize = 8;
    const SEPARATOR_SIZE: usize = 1;
    const ADDR_LEN_SIZE: usize = 1;

    LANG_SIZE + 2 * COORD_SIZE + 2 * SEPARATOR_SIZE + ADDR_LEN_SIZE
  }
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;

  use crate::{
    DequeStorage,
    cache_key::CacheKey,
    storage::{Storage, StorageStrategy, deque::DeqeueStorageItem},
  };

  const SIZE: usize = 30;
  // unknown-00
  const ADDRESS_LEN: usize = 10;

  fn create_test_storage() -> (Storage, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap();
    let storage = Storage::try_new(tmp.path()).unwrap();
    (storage, tmp)
  }

  #[test]
  fn deque_read() {
    let mut deque_storage = DequeStorage::default();
    let (mut storage, _tmp) = create_test_storage();

    deque_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    deque_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(deque_storage.cache_keys.len(), 2);
    assert_eq!(deque_storage.data.len(), 2);

    deque_storage.flush(&mut storage).unwrap();

    drop(deque_storage);

    // Create a new instance
    let mut deque_storage = DequeStorage::default();
    deque_storage.read(&mut storage).unwrap();

    assert_eq!(deque_storage.cache_keys.len(), 2);
    assert_eq!(deque_storage.data.len(), 2);
  }

  #[test]
  fn deque_insertion() {
    let mut deque_storage = DequeStorage::default();

    deque_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    deque_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(deque_storage.cache_keys.len(), 2);
    assert_eq!(deque_storage.data.len(), 2);
  }

  #[test]
  fn deque_deletion() {
    let mut deque_storage = DequeStorage::default();
    let (mut storage, _tmp) = create_test_storage();

    // Mock the data
    for i in 1..=30 {
      deque_storage
        .insert(
          CacheKey::try_new(48.1645819 + i as f64, 17.1847104 + i as f64, "en").unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    assert_eq!(deque_storage.cache_keys.len(), 30);
    assert_eq!(deque_storage.data.len(), 30);

    deque_storage.flush(&mut storage).unwrap();
    deque_storage.evict(&mut storage).unwrap();

    assert_eq!(deque_storage.cache_keys.len(), 20);
    assert_eq!(deque_storage.data.len(), 20);

    let first_record = deque_storage.first().unwrap();
    let last_record = deque_storage.last().unwrap();

    assert_eq!(first_record.1, "unknown-30");
    assert_eq!(last_record.1, "unknown-11");
  }

  #[test]
  fn deque_memory_size() {
    let mut deque_storage = DequeStorage::default();

    for i in 0..SIZE {
      deque_storage
        .insert(
          CacheKey::try_new(48.1645819 + i as f64, 17.1847104 + i as f64, "en").unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    assert_eq!(
      deque_storage.get_in_memory_size(),
      DeqeueStorageItem::len() * SIZE + ADDRESS_LEN * SIZE
    );
  }

  #[test]
  fn deque_memory_size_with_eviction() {
    let mut deque_storage = DequeStorage::default();
    let (mut storage, _tmp) = create_test_storage();

    // Mock the data
    for i in 0..SIZE {
      deque_storage
        .insert(
          CacheKey::try_new(48.1645819 + i as f64, 17.1847104 + i as f64, "en").unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    const MEMORY_SIZE: usize = DeqeueStorageItem::len() * SIZE + ADDRESS_LEN * SIZE;

    const MEMORY_SIZE_OF_DELETED_RECORDS: usize = DeqeueStorageItem::len()
      * DequeStorage::ON_DELETE_ITEMS_COUNT
      + ADDRESS_LEN * DequeStorage::ON_DELETE_ITEMS_COUNT;

    assert_eq!(deque_storage.get_in_memory_size(), MEMORY_SIZE);

    deque_storage.flush(&mut storage).unwrap();
    deque_storage.evict(&mut storage).unwrap();

    assert_eq!(
      deque_storage.get_in_memory_size(),
      MEMORY_SIZE - MEMORY_SIZE_OF_DELETED_RECORDS
    );
  }
}
