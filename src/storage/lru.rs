use lru::LruCache;

use crate::{
  cache_key::{CacheKey, CacheKeyRaw},
  storage::{Address, Storage, StorageStrategy},
};

#[cfg(feature = "testing")]
use crate::storage::StorageStrategyWithCapacity;

use super::deque::DequeStorageItem;

/// LruStorage is a persistence storage technique using LRU eviction
/// via the `lru` crate's `LruCache`.
///
/// Items are ordered by use: the most recently used (inserted or accessed)
/// sits at the front, and the least recently used sits at the back.
/// When eviction is triggered, the least recently used items are removed first.
pub struct LruStorage {
  data: LruCache<CacheKey, Address>,
  memory_size: usize,
  memory_max_size: usize,
}

impl Default for LruStorage {
  fn default() -> Self {
    Self {
      data: LruCache::unbounded(),
      memory_size: 0,
      memory_max_size: 1000,
    }
  }
}

impl StorageStrategy for LruStorage {
  const ON_DELETE_ITEMS_COUNT_PERCENTAGE: usize = 10;

  fn insert(
    &mut self,
    cache_key: CacheKey,
    address: Address,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.memory_size += DequeStorageItem::len() + address.len();
    self.data.put(cache_key, address);
    Ok(())
  }

  fn get(&mut self, cache_key: &CacheKey) -> Option<&Address> {
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
      .data
      .iter()
      .flat_map(|(cache_key, address)| {
        DequeStorageItem::from_cache_key(cache_key, address).to_bytes()
      })
      .collect()
  }

  fn read(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    let bytes = storage.read()?;
    let mut entries = Vec::new();
    let mut pos = 0;

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
      entries.push((cache_key, address));
    }

    // Insert in reverse to restore LRU order (file stores newest-first)
    for (cache_key, address) in entries.into_iter().rev() {
      self.data.put(cache_key, address);
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
    let to_remove = self.on_delete_items_count();

    let mut removed_memory = 0;
    for _ in 0..to_remove {
      if let Some((_key, address)) = self.data.pop_lru() {
        removed_memory += DequeStorageItem::len() + address.len();
      } else {
        break;
      }
    }

    self.memory_size = self.memory_size.saturating_sub(removed_memory);
    self.flush(storage)?;
    self.memory_size = storage.len()? as usize;

    Ok(())
  }

  fn in_memory_record_count(&self) -> usize {
    self.data.len()
  }
}

#[cfg(feature = "testing")]
impl StorageStrategyWithCapacity for LruStorage {
  fn with_capacity(_capacity: usize) -> Self {
    Self {
      data: LruCache::unbounded(),
      memory_size: 0,
      memory_max_size: 1000,
    }
  }
}

impl LruStorage {
  #[allow(unused)]
  fn first(&self) -> Option<(&CacheKey, &Address)> {
    self.data.iter().next()
  }

  #[allow(unused)]
  fn last(&self) -> Option<(&CacheKey, &Address)> {
    self.data.iter().next_back()
  }

  fn on_delete_items_count(&self) -> usize {
    if self.memory_max_size == 0 || self.data.is_empty() {
      return 0;
    }

    let usage_percentage = (self.memory_size * 100) / self.memory_max_size;
    if usage_percentage >= Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE {
      let count = (self.data.len() * Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE) / 100;
      count.min(self.data.len())
    } else {
      0
    }
  }
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;

  use crate::{
    LruStorage,
    cache_key::CacheKey,
    storage::{Storage, StorageStrategy, deque::DequeStorageItem},
  };

  const SIZE: usize = 100;
  const ADDRESS_LEN: usize = 10;

  fn create_test_storage() -> (Storage, NamedTempFile) {
    let tmp = NamedTempFile::new().unwrap();
    let storage = Storage::try_new(tmp.path()).unwrap();
    (storage, tmp)
  }

  fn create_test_lru_storage() -> LruStorage {
    let mut lru_storage = LruStorage::default();
    lru_storage.memory_max_size(1000);
    lru_storage
  }

  #[test]
  fn lru_read() {
    let mut lru_storage = create_test_lru_storage();
    let (mut storage, _tmp) = create_test_storage();

    lru_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lru_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(lru_storage.in_memory_record_count(), 2);

    lru_storage.flush(&mut storage).unwrap();

    drop(lru_storage);

    let mut lru_storage = LruStorage::default();
    lru_storage.read(&mut storage).unwrap();

    assert_eq!(lru_storage.in_memory_record_count(), 2);
  }

  #[test]
  fn lru_insertion() {
    let mut lru_storage = create_test_lru_storage();

    lru_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lru_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(lru_storage.in_memory_record_count(), 2);
  }

  #[test]
  fn lru_deletion() {
    let mut lru_storage = create_test_lru_storage();
    let (mut storage, _tmp) = create_test_storage();

    for i in 1..=SIZE {
      lru_storage
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

    assert_eq!(lru_storage.in_memory_record_count(), SIZE);

    lru_storage.flush(&mut storage).unwrap();
    lru_storage.evict(&mut storage).unwrap();

    assert_eq!(lru_storage.in_memory_record_count(), 90);

    let first_record = lru_storage.first().unwrap();
    let last_record = lru_storage.last().unwrap();

    assert_eq!(first_record.1, "unknown-100");
    assert_eq!(last_record.1, "unknown-11");
  }

  #[test]
  fn lru_memory_size() {
    let mut lru_storage = create_test_lru_storage();

    for i in 0..SIZE {
      lru_storage
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
      lru_storage.get_in_memory_size(),
      DequeStorageItem::len() * SIZE + ADDRESS_LEN * SIZE
    );
  }

  #[test]
  fn lru_memory_size_with_eviction() {
    let mut lru_storage = create_test_lru_storage();
    let (mut storage, _tmp) = create_test_storage();

    for i in 0..SIZE {
      lru_storage
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

    let on_delete_items_count = lru_storage.on_delete_items_count();

    let memory_size_of_deleted_records =
      DequeStorageItem::len() * on_delete_items_count + ADDRESS_LEN * on_delete_items_count;

    assert_eq!(lru_storage.get_in_memory_size(), MEMORY_SIZE);

    lru_storage.flush(&mut storage).unwrap();
    lru_storage.evict(&mut storage).unwrap();

    assert_eq!(
      lru_storage.get_in_memory_size(),
      MEMORY_SIZE - memory_size_of_deleted_records
    );
  }

  #[test]
  fn lru_read_preserves_order() {
    let mut lru_storage = create_test_lru_storage();
    let (mut storage, _tmp) = create_test_storage();

    lru_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lru_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    // Order: MRU = Prague, LRU = Bratislava
    assert_eq!(lru_storage.first().unwrap().1, "Prague, Czechia");
    assert_eq!(lru_storage.last().unwrap().1, "Bratislava, Slovakia");

    lru_storage.flush(&mut storage).unwrap();
    drop(lru_storage);

    let mut lru_storage = LruStorage::default();
    lru_storage.read(&mut storage).unwrap();

    // Order must be restored: MRU = Prague, LRU = Bratislava
    let first = lru_storage.first().unwrap();
    let last = lru_storage.last().unwrap();

    assert_eq!(first.1, "Prague, Czechia");
    assert_eq!(last.1, "Bratislava, Slovakia");
  }

  #[test]
  fn lru_get_promotes_item() {
    let mut lru_storage = create_test_lru_storage();

    lru_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava".to_string(),
      )
      .unwrap();

    lru_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague".to_string(),
      )
      .unwrap();

    lru_storage
      .insert(
        CacheKey::try_new(51.5074, -0.1278, "en").unwrap(),
        "London".to_string(),
      )
      .unwrap();

    // Order: MRU = London, then Prague, LRU = Bratislava
    assert_eq!(lru_storage.first().unwrap().1, "London");
    assert_eq!(lru_storage.last().unwrap().1, "Bratislava");

    // Access Bratislava — promotes it to MRU
    lru_storage.get(&CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap());

    // Order: MRU = Bratislava, then London, LRU = Prague
    assert_eq!(lru_storage.first().unwrap().1, "Bratislava");
    assert_eq!(lru_storage.last().unwrap().1, "Prague");
  }

  /// Verifies serialization compatibility with DequeStorage
  #[test]
  fn lru_serialization_compatible_with_deque() {
    use crate::DequeStorage;

    let (mut storage, _tmp) = create_test_storage();
    let mut deque = DequeStorage::default();
    deque.memory_max_size(1000);

    deque
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    deque
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    // Write with Deque, read with Lru
    deque.flush(&mut storage).unwrap();
    drop(deque);

    let mut lru = LruStorage::default();
    lru.memory_max_size(1000);
    lru.read(&mut storage).unwrap();

    assert_eq!(lru.in_memory_record_count(), 2);
    assert_eq!(
      lru.get(&CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap()),
      Some(&"Bratislava, Slovakia".to_string())
    );
    assert_eq!(
      lru.get(&CacheKey::try_new(50.073658, 14.418540, "en").unwrap()),
      Some(&"Prague, Czechia".to_string())
    );

    // Write with Lru, read with Deque
    lru.flush(&mut storage).unwrap();
    drop(lru);

    let mut deque = DequeStorage::default();
    deque.memory_max_size(1000);
    deque.read(&mut storage).unwrap();

    assert_eq!(deque.in_memory_record_count(), 2);
  }
}
