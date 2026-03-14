use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
  cache_key::{CacheKey, CacheKeyRaw},
  storage::{Address, Storage, StorageStrategy},
};

#[cfg(feature = "testing")]
use crate::storage::StorageStrategyWithCapacity;

/// DequeStorage is a simple persistence storage technique
/// which consists of a `data` field and a `cache_keys` field.
///
/// Field `cache_keys` is essential for remembering the insertion
/// order of each `CacheKey` - the oldest inserted key sits at
/// the back of the `VecDeque`.
#[derive(Default)]
pub struct DequeStorage {
  /// Maps each `CacheKey` to its corresponding `Address` in memory
  data: HashMap<CacheKey, Address>,
  /// Tracks insertion order of keys (newest at the front, oldest at the back)
  cache_keys: VecDeque<CacheKey>,
  /// Current total memory usage in bytes
  memory_size: usize,
  /// Max allowed memory usage
  memory_max_size: usize,
}

impl StorageStrategy for DequeStorage {
  const ON_DELETE_ITEMS_COUNT_PERCENTAGE: usize = 10;

  fn insert(
    &mut self,
    cache_key: CacheKey,
    address: Address,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.memory_size += DequeStorageItem::len() + address.len();
    self.cache_keys.push_front(cache_key.clone());
    self.data.insert(cache_key, address);
    Ok(())
  }

  fn get(&self, cache_key: &CacheKey) -> Option<&Address> {
    self.data.get(cache_key)
  }

  fn memory_max_size(&mut self, size: usize) {
    self.memory_max_size = size;
  }

  fn get_in_memory_size(&self) -> usize {
    self.memory_size
  }

  fn as_bytes(&self) -> Vec<u8> {
    self
      .cache_keys
      .iter()
      .filter_map(|cache_key| {
        self
          .data
          .get(cache_key)
          .map(|address| DequeStorageItem::from_cache_key(cache_key, address).to_bytes())
      })
      .flatten()
      .collect()
  }

  fn read(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    let bytes = storage.read()?;
    let mut pos = 0;

    // Fixed-width key portion: total item size minus the 1-byte address-length prefix
    const KEY_LEN: usize = DequeStorageItem::len() - 1;

    while pos + KEY_LEN <= bytes.len() {
      let key_bytes: [u8; KEY_LEN] = bytes[pos..pos + KEY_LEN].try_into().unwrap();
      let cache_key_raw = CacheKeyRaw(key_bytes);
      pos += KEY_LEN;

      let addr_len = bytes[pos] as usize;
      pos += 1;

      let address = String::from_utf8_lossy(&bytes[pos..pos + addr_len]).into_owned();
      pos += addr_len;

      let cache_key: CacheKey = cache_key_raw.into();
      self.memory_size += DequeStorageItem::len() + addr_len;
      self.data.insert(cache_key.clone(), address);
      // push_back preserves the on-disk order (newest first → oldest last)
      self.cache_keys.push_back(cache_key);
    }

    Ok(())
  }

  fn flush(&self, storage: &mut Storage) -> std::io::Result<()> {
    storage.truncate_and_write(&self.as_bytes())
  }

  fn evict_if_needed(&mut self, storage: &mut Storage, address_len: usize) -> std::io::Result<()> {
    if DequeStorageItem::len() + address_len > self.memory_size {
      self.evict(storage)?;
    }
    Ok(())
  }

  fn evict(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    self.cache_keys.truncate(
      self
        .cache_keys
        .len()
        .saturating_sub(self.on_delete_items_count()),
    );

    let remaining: HashSet<&CacheKey> = self.cache_keys.iter().collect();
    self.data.retain(|key, _| remaining.contains(key));

    self.flush(storage)?;

    // Sync memory_size to the actual serialized size on disk
    self.memory_size = storage.len()? as usize;

    Ok(())
  }

  fn in_memory_record_count(&self) -> usize {
    self.cache_keys.len()
  }
}

#[cfg(feature = "testing")]
impl StorageStrategyWithCapacity for DequeStorage {
  fn with_capacity(capacity: usize) -> Self {
    Self {
      data: HashMap::with_capacity(capacity),
      cache_keys: VecDeque::with_capacity(capacity),
      memory_size: 0,
      memory_max_size: 1000,
    }
  }
}

impl DequeStorage {
  /// Returns the most recently inserted record.
  #[allow(unused)]
  fn first(&self) -> Option<(&CacheKey, &Address)> {
    let cache_key = self.cache_keys.front()?;
    let address = self.data.get(cache_key)?;
    Some((cache_key, address))
  }

  /// Returns the oldest inserted record.
  #[allow(unused)]
  fn last(&self) -> Option<(&CacheKey, &Address)> {
    let cache_key = self.cache_keys.back()?;
    let address = self.data.get(cache_key)?;
    Some((cache_key, address))
  }

  /// Returns the number of items to evict when memory limit is exceeded.
  /// Calculated as a percentage of `memory_size`, capped at the current cache size.
  fn on_delete_items_count(&self) -> usize {
    if self.memory_max_size == 0 || self.cache_keys.is_empty() {
      return 0;
    }

    let usage_percentage = (self.memory_size * 100) / self.memory_max_size;
    if usage_percentage >= Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE {
      let count = (self.cache_keys.len() * Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE) / 100;
      count.min(self.cache_keys.len())
    } else {
      0
    }
  }
}

pub(crate) struct DequeStorageItem {
  // Raw cache key
  cache_key_raw: CacheKeyRaw,
  /// Address string length (TODO:limited to 255 length)
  address_len: u8,
  /// Geocoded address string
  address: Address,
}

impl DequeStorageItem {
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

  /// Size of the fixed-width portion of a record in bytes.
  /// 2 (lang) + 2 × 8 (lat/lng) + 2 (separators) + 1 (address length prefix)
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
    storage::{Storage, StorageStrategy, deque::DequeStorageItem},
  };

  const SIZE: usize = 100;
  // unknown-00
  const ADDRESS_LEN: usize = 10;

  fn create_test_storage() -> (Storage, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap();
    let storage = Storage::try_new(tmp.path()).unwrap();
    (storage, tmp)
  }

  fn create_test_deque_storage() -> DequeStorage {
    let mut deque_storage = DequeStorage::default();
    deque_storage.memory_max_size(1000);
    deque_storage
  }

  #[test]
  fn deque_read() {
    let mut deque_storage = create_test_deque_storage();
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
    let mut deque_storage = create_test_deque_storage();

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
    let mut deque_storage = create_test_deque_storage();
    let (mut storage, _tmp) = create_test_storage();

    // Mock the data
    for i in 1..=SIZE {
      deque_storage
        .insert(
          CacheKey::try_new(
            48.1645819 + (i as f64 * 0.01) as f64,
            17.1847104 + (i as f64 * 0.01) as f64,
            "en",
          )
          .unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    assert_eq!(deque_storage.cache_keys.len(), SIZE);
    assert_eq!(deque_storage.data.len(), SIZE);

    deque_storage.flush(&mut storage).unwrap();
    deque_storage.evict(&mut storage).unwrap();

    assert_eq!(deque_storage.cache_keys.len(), 90);
    assert_eq!(deque_storage.data.len(), 90);

    let first_record = deque_storage.first().unwrap();
    let last_record = deque_storage.last().unwrap();

    assert_eq!(first_record.1, "unknown-100");
    assert_eq!(last_record.1, "unknown-11");
  }

  #[test]
  fn deque_memory_size() {
    let mut deque_storage = create_test_deque_storage();

    for i in 0..SIZE {
      deque_storage
        .insert(
          CacheKey::try_new(
            48.1645819 + (i as f64 * 0.01) as f64,
            17.1847104 + (i as f64 * 0.01) as f64,
            "en",
          )
          .unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    assert_eq!(
      deque_storage.get_in_memory_size(),
      DequeStorageItem::len() * SIZE + ADDRESS_LEN * SIZE
    );
  }

  #[test]
  fn deque_memory_size_with_eviction() {
    let mut deque_storage = create_test_deque_storage();
    let (mut storage, _tmp) = create_test_storage();

    // Mock the data
    for i in 0..SIZE {
      deque_storage
        .insert(
          CacheKey::try_new(
            48.1645819 + (i as f64 * 0.01) as f64,
            17.1847104 + (i as f64 * 0.01) as f64,
            "en",
          )
          .unwrap(),
          format!("unknown-{:02}", i),
        )
        .unwrap();
    }

    const MEMORY_SIZE: usize = DequeStorageItem::len() * SIZE + ADDRESS_LEN * SIZE;

    let on_delete_items_count = deque_storage.on_delete_items_count();

    let memory_size_of_deleted_records =
      DequeStorageItem::len() * on_delete_items_count + ADDRESS_LEN * on_delete_items_count;

    assert_eq!(deque_storage.get_in_memory_size(), MEMORY_SIZE);

    deque_storage.flush(&mut storage).unwrap();
    deque_storage.evict(&mut storage).unwrap();

    assert_eq!(
      deque_storage.get_in_memory_size(),
      MEMORY_SIZE - memory_size_of_deleted_records
    );
  }
}
