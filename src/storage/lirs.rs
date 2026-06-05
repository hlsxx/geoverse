use std::collections::HashMap;

use crate::{
  cache_key::{CacheKey, CacheKeyRaw},
  storage::{Address, Storage, StorageStrategy},
};

#[cfg(feature = "testing")]
use crate::storage::StorageStrategyWithCapacity;

use super::deque::DequeStorageItem;

const LIRS_LIMIT_PERCENTAGE: usize = 95;
const AVG_ADDRESS_LEN: usize = 20;

pub struct LirsStorage {
  slots: Vec<Option<LirsSlot>>,
  key_to_idx: HashMap<CacheKey, usize>,
  free_list: Vec<usize>,

  stack_head: Option<usize>,
  stack_tail: Option<usize>,

  queue_head: Option<usize>,
  queue_tail: Option<usize>,

  lir_count: usize,
  lir_limit: usize,
  resident_count: usize,

  memory_size: usize,
  memory_max_size: usize,
}

struct LirsSlot {
  key: CacheKey,
  address: Address,
  prev: Option<usize>,
  next: Option<usize>,
  queue_prev: Option<usize>,
  queue_next: Option<usize>,
  is_lir: bool,
  resident: bool,
  in_stack: bool,
  in_queue: bool,
}

impl Default for LirsStorage {
  fn default() -> Self {
    Self {
      slots: Vec::new(),
      key_to_idx: HashMap::new(),
      free_list: Vec::new(),
      stack_head: None,
      stack_tail: None,
      queue_head: None,
      queue_tail: None,
      lir_count: 0,
      lir_limit: 95,
      resident_count: 0,
      memory_size: 0,
      memory_max_size: 10 * 1024 * 1024,
    }
  }
}

impl StorageStrategy for LirsStorage {
  const ON_DELETE_ITEMS_COUNT_PERCENTAGE: usize = 10;

  fn insert(
    &mut self,
    cache_key: CacheKey,
    address: Address,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(&idx) = self.key_to_idx.get(&cache_key) {
      let (was_resident, old_addr_len, new_addr_len) = {
        let slot = self.slots[idx].as_mut().unwrap();
        let was_resident = slot.resident;
        let old_addr_len = slot.address.len();
        slot.address.clone_from(&address);
        (was_resident, old_addr_len, address.len())
      };

      self.memory_size = self
        .memory_size
        .wrapping_add(new_addr_len)
        .wrapping_sub(old_addr_len);
      if !was_resident {
        self.slots[idx].as_mut().unwrap().resident = true;
        self.resident_count += 1;
        self.memory_size += DequeStorageItem::len();
      }

      self.move_to_stack_top(idx);

      let should_promote = {
        let slot = self.slots[idx].as_ref().unwrap();
        !slot.is_lir && slot.resident
      };
      if should_promote {
        self.slots[idx].as_mut().unwrap().is_lir = true;
        self.remove_from_queue(idx);
        self.lir_count += 1;
      }

      self.prune_stack();
    } else {
      let idx = self.allocate_slot(cache_key, address);
      self.resident_count += 1;
      self.memory_size += DequeStorageItem::len() + self.slots[idx].as_ref().unwrap().address.len();

      self.push_stack_top(idx);

      if self.lir_count < self.lir_limit {
        self.slots[idx].as_mut().unwrap().is_lir = true;
        self.lir_count += 1;
      } else {
        self.slots[idx].as_mut().unwrap().is_lir = false;
        self.enqueue(idx);
      }

      self.prune_stack();
    }

    Ok(())
  }

  fn get(&mut self, cache_key: &CacheKey) -> Option<&Address> {
    let idx = *self.key_to_idx.get(cache_key)?;

    if self.slots[idx].as_ref().unwrap().in_stack {
      self.move_to_stack_top(idx);
    } else {
      self.push_stack_top(idx);
    }

    let (should_promote, resident) = {
      let slot = self.slots[idx].as_ref().unwrap();
      (!slot.is_lir && slot.resident, slot.resident)
    };

    if should_promote {
      self.slots[idx].as_mut().unwrap().is_lir = true;
      self.remove_from_queue(idx);
      self.lir_count += 1;
    }

    if !resident {
      return None;
    }

    self.prune_stack();

    self.slots[idx].as_ref().map(|s| &s.address)
  }

  fn memory_max_size(&mut self, size: usize) {
    self.memory_max_size = size;
    self.update_lir_limit();
  }

  fn get_in_memory_size(&self) -> usize {
    self.memory_size
  }

  fn as_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Some(head) = self.stack_head {
      let mut cur = Some(head);
      while let Some(idx) = cur {
        let slot = self.slots[idx].as_ref().unwrap();
        if slot.resident {
          bytes.extend(DequeStorageItem::from_cache_key(&slot.key, &slot.address).to_bytes());
        }
        cur = slot.next;
      }
    }
    bytes
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
      entries.push((cache_key, address));
    }

    // Insert in reverse: file stores newest-first, so reversing gives
    // oldest-first, and push_stack_top builds the stack in correct order
    for (cache_key, address) in entries.into_iter().rev() {
      let idx = self.allocate_slot(cache_key, address);
      self.resident_count += 1;
      self.memory_size += DequeStorageItem::len() + self.slots[idx].as_ref().unwrap().address.len();

      self.push_stack_top(idx);

      if self.lir_count < self.lir_limit {
        self.slots[idx].as_mut().unwrap().is_lir = true;
        self.lir_count += 1;
      } else {
        self.slots[idx].as_mut().unwrap().is_lir = false;
        self.enqueue(idx);
      }
    }

    self.prune_stack();
    Ok(())
  }

  fn flush(&self, storage: &mut Storage) -> std::io::Result<()> {
    storage.truncate_and_write(&self.as_bytes())
  }

  fn evict_if_needed(&mut self, _storage: &mut Storage, address_len: usize) -> std::io::Result<()> {
    let item_size = DequeStorageItem::len() + address_len;
    while self.queue_head.is_some() && self.memory_size + item_size > self.memory_max_size {
      self.evict_one();
    }
    self.flush(_storage)?;
    Ok(())
  }

  fn evict(&mut self, storage: &mut Storage) -> std::io::Result<()> {
    let to_remove = self.on_delete_items_count();
    for _ in 0..to_remove {
      if self.queue_head.is_none() {
        break;
      }
      self.evict_one();
    }
    self.flush(storage)?;
    self.memory_size = storage.len()? as usize;
    Ok(())
  }

  fn in_memory_record_count(&self) -> usize {
    self.resident_count
  }
}

#[cfg(feature = "testing")]
impl StorageStrategyWithCapacity for LirsStorage {
  fn with_capacity(capacity: usize) -> Self {
    Self {
      slots: Vec::with_capacity(capacity),
      key_to_idx: HashMap::with_capacity(capacity),
      free_list: Vec::new(),
      stack_head: None,
      stack_tail: None,
      queue_head: None,
      queue_tail: None,
      lir_count: 0,
      lir_limit: (capacity * LIRS_LIMIT_PERCENTAGE) / 100,
      resident_count: 0,
      memory_size: 0,
      memory_max_size: capacity * (DequeStorageItem::len() + AVG_ADDRESS_LEN),
    }
  }
}

impl LirsStorage {
  #[allow(unused)]
  fn first(&self) -> Option<(&CacheKey, &Address)> {
    let mut cur = self.stack_head;
    while let Some(idx) = cur {
      let slot = self.slots[idx].as_ref()?;
      if slot.resident {
        return Some((&slot.key, &slot.address));
      }
      cur = slot.next;
    }
    None
  }

  #[allow(unused)]
  fn last(&self) -> Option<(&CacheKey, &Address)> {
    let mut cur = self.stack_tail;
    while let Some(idx) = cur {
      let slot = self.slots[idx].as_ref()?;
      if slot.resident {
        return Some((&slot.key, &slot.address));
      }
      cur = slot.prev;
    }
    None
  }

  fn on_delete_items_count(&self) -> usize {
    if self.memory_max_size == 0 || self.resident_count == 0 {
      return 0;
    }

    let usage_percentage = (self.memory_size * 100) / self.memory_max_size;
    if usage_percentage >= Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE {
      let count = (self.resident_count * Self::ON_DELETE_ITEMS_COUNT_PERCENTAGE) / 100;
      count.min(self.resident_count)
    } else {
      0
    }
  }

  fn allocate_slot(&mut self, key: CacheKey, address: Address) -> usize {
    if let Some(free) = self.free_list.pop() {
      self.slots[free] = Some(LirsSlot {
        key,
        address,
        prev: None,
        next: None,
        queue_prev: None,
        queue_next: None,
        is_lir: false,
        resident: true,
        in_stack: false,
        in_queue: false,
      });
      self
        .key_to_idx
        .insert(self.slots[free].as_ref().unwrap().key.clone(), free);
      free
    } else {
      let idx = self.slots.len();
      self.slots.push(Some(LirsSlot {
        key,
        address,
        prev: None,
        next: None,
        queue_prev: None,
        queue_next: None,
        is_lir: false,
        resident: true,
        in_stack: false,
        in_queue: false,
      }));
      self
        .key_to_idx
        .insert(self.slots[idx].as_ref().unwrap().key.clone(), idx);
      idx
    }
  }

  fn free_slot(&mut self, idx: usize) {
    if let Some(slot) = &self.slots[idx] {
      self.key_to_idx.remove(&slot.key);
    }
    self.slots[idx] = None;
    self.free_list.push(idx);
  }

  fn push_stack_top(&mut self, idx: usize) {
    let slot = self.slots[idx].as_mut().unwrap();
    slot.prev = None;
    slot.next = self.stack_head;
    slot.in_stack = true;

    if let Some(head) = self.stack_head {
      self.slots[head].as_mut().unwrap().prev = Some(idx);
    }
    self.stack_head = Some(idx);
    if self.stack_tail.is_none() {
      self.stack_tail = Some(idx);
    }
  }

  fn move_to_stack_top(&mut self, idx: usize) {
    if self.stack_head == Some(idx) {
      return;
    }

    let (prev, next) = {
      let slot = self.slots[idx].as_ref().unwrap();
      (slot.prev, slot.next)
    };

    if let Some(p) = prev
      && let Some(slot) = &mut self.slots[p]
    {
      slot.next = next;
    }
    if let Some(n) = next
      && let Some(slot) = &mut self.slots[n]
    {
      slot.prev = prev;
    }
    if self.stack_tail == Some(idx) {
      self.stack_tail = prev;
    }

    {
      let slot = self.slots[idx].as_mut().unwrap();
      slot.prev = None;
      slot.next = self.stack_head;
    }
    if let Some(head) = self.stack_head {
      self.slots[head].as_mut().unwrap().prev = Some(idx);
    }
    self.stack_head = Some(idx);
  }

  fn enqueue(&mut self, idx: usize) {
    self.slots[idx].as_mut().unwrap().queue_prev = self.queue_tail;
    self.slots[idx].as_mut().unwrap().queue_next = None;
    self.slots[idx].as_mut().unwrap().in_queue = true;

    if let Some(tail) = self.queue_tail {
      self.slots[tail].as_mut().unwrap().queue_next = Some(idx);
    }
    self.queue_tail = Some(idx);
    if self.queue_head.is_none() {
      self.queue_head = Some(idx);
    }
  }

  fn dequeue(&mut self) -> Option<usize> {
    let head = self.queue_head?;
    let next = self.slots[head].as_ref().unwrap().queue_next;

    if let Some(n) = next {
      self.slots[n].as_mut().unwrap().queue_prev = None;
    }
    self.queue_head = next;
    if self.queue_tail == Some(head) {
      self.queue_tail = None;
    }

    self.slots[head].as_mut().unwrap().in_queue = false;
    Some(head)
  }

  fn remove_from_queue(&mut self, idx: usize) {
    if !self.slots[idx].as_ref().unwrap().in_queue {
      return;
    }
    let (prev, next) = {
      let slot = self.slots[idx].as_ref().unwrap();
      (slot.queue_prev, slot.queue_next)
    };

    if let Some(p) = prev {
      self.slots[p].as_mut().unwrap().queue_next = next;
    }
    if let Some(n) = next {
      self.slots[n].as_mut().unwrap().queue_prev = prev;
    }
    if self.queue_head == Some(idx) {
      self.queue_head = next;
    }
    if self.queue_tail == Some(idx) {
      self.queue_tail = prev;
    }
    self.slots[idx].as_mut().unwrap().in_queue = false;
  }

  fn evict_one(&mut self) -> Option<usize> {
    let victim = self.dequeue()?;
    let slot = self.slots[victim].as_mut().unwrap();
    slot.resident = false;
    self.resident_count = self.resident_count.saturating_sub(1);

    let item_size = DequeStorageItem::len() + slot.address.len();
    self.memory_size = self.memory_size.saturating_sub(item_size);

    Some(victim)
  }

  fn prune_stack(&mut self) {
    while let Some(tail) = self.stack_tail {
      let (is_lir, resident, prev) = match &self.slots[tail] {
        Some(s) => (s.is_lir, s.resident, s.prev),
        None => break,
      };

      if !resident && !is_lir {
        if let Some(p) = prev {
          self.slots[p].as_mut().unwrap().next = None;
        }
        self.stack_tail = prev;
        if self.stack_head == Some(tail) {
          self.stack_head = None;
        }
        self.free_slot(tail);
      } else if is_lir && self.lir_count > self.lir_limit {
        self.slots[tail].as_mut().unwrap().is_lir = false;
        self.lir_count = self.lir_count.saturating_sub(1);
        if !self.slots[tail].as_ref().unwrap().in_queue {
          self.enqueue(tail);
        }
      } else {
        break;
      }
    }
  }

  fn update_lir_limit(&mut self) {
    let avg_item = DequeStorageItem::len() + AVG_ADDRESS_LEN;
    let max_items = (self.memory_max_size / avg_item).max(10);
    self.lir_limit = (max_items * LIRS_LIMIT_PERCENTAGE) / 100;
  }
}

#[cfg(test)]
mod tests {
  use tempfile::NamedTempFile;

  use crate::{
    LirsStorage,
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

  fn create_test_lirs_storage() -> LirsStorage {
    let mut lirs_storage = LirsStorage::default();
    lirs_storage.memory_max_size(1000);
    lirs_storage
  }

  #[test]
  fn lirs_read() {
    let mut lirs_storage = create_test_lirs_storage();
    let (mut storage, _tmp) = create_test_storage();

    lirs_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lirs_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(lirs_storage.in_memory_record_count(), 2);

    lirs_storage.flush(&mut storage).unwrap();

    drop(lirs_storage);

    let mut lirs_storage = LirsStorage::default();
    lirs_storage.read(&mut storage).unwrap();

    assert_eq!(lirs_storage.in_memory_record_count(), 2);
  }

  #[test]
  fn lirs_insertion() {
    let mut lirs_storage = create_test_lirs_storage();

    lirs_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lirs_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(lirs_storage.in_memory_record_count(), 2);
  }

  #[test]
  fn lirs_deletion() {
    let mut lirs_storage = create_test_lirs_storage();
    let (mut storage, _tmp) = create_test_storage();

    for i in 1..=SIZE {
      lirs_storage
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

    assert_eq!(lirs_storage.in_memory_record_count(), SIZE);

    lirs_storage.flush(&mut storage).unwrap();
    lirs_storage.evict(&mut storage).unwrap();

    assert_eq!(lirs_storage.in_memory_record_count(), 90);

    let first_record = lirs_storage.first().unwrap();
    assert_eq!(first_record.1, "unknown-100");

    // LIRS evicts from the HIR queue (items 23-32, the first HIR blocks),
    // not from the tail. Item 1 stays at the stack bottom as a LIR resident.
    let last_record = lirs_storage.last().unwrap();
    assert_eq!(last_record.1, "unknown-01");
  }

  #[test]
  fn lirs_memory_size() {
    let mut lirs_storage = create_test_lirs_storage();

    for i in 0..SIZE {
      lirs_storage
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
      lirs_storage.get_in_memory_size(),
      DequeStorageItem::len() * SIZE + ADDRESS_LEN * SIZE
    );
  }

  #[test]
  fn lirs_memory_size_with_eviction() {
    let mut lirs_storage = create_test_lirs_storage();
    let (mut storage, _tmp) = create_test_storage();

    for i in 0..SIZE {
      lirs_storage
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

    let on_delete_items_count = lirs_storage.on_delete_items_count();

    let memory_size_of_deleted_records =
      DequeStorageItem::len() * on_delete_items_count + ADDRESS_LEN * on_delete_items_count;

    assert_eq!(lirs_storage.get_in_memory_size(), MEMORY_SIZE);

    lirs_storage.flush(&mut storage).unwrap();
    lirs_storage.evict(&mut storage).unwrap();

    assert_eq!(
      lirs_storage.get_in_memory_size(),
      MEMORY_SIZE - memory_size_of_deleted_records
    );
  }

  #[test]
  fn lirs_read_preserves_order() {
    let mut lirs_storage = create_test_lirs_storage();
    let (mut storage, _tmp) = create_test_storage();

    lirs_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava, Slovakia".to_string(),
      )
      .unwrap();

    lirs_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague, Czechia".to_string(),
      )
      .unwrap();

    assert_eq!(lirs_storage.first().unwrap().1, "Prague, Czechia");
    assert_eq!(lirs_storage.last().unwrap().1, "Bratislava, Slovakia");

    lirs_storage.flush(&mut storage).unwrap();
    drop(lirs_storage);

    let mut lirs_storage = LirsStorage::default();
    lirs_storage.read(&mut storage).unwrap();

    let first = lirs_storage.first().unwrap();
    let last = lirs_storage.last().unwrap();

    assert_eq!(first.1, "Prague, Czechia");
    assert_eq!(last.1, "Bratislava, Slovakia");
  }

  #[test]
  fn lirs_get_promotes_item() {
    let mut lirs_storage = create_test_lirs_storage();

    lirs_storage
      .insert(
        CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap(),
        "Bratislava".to_string(),
      )
      .unwrap();

    lirs_storage
      .insert(
        CacheKey::try_new(50.073658, 14.418540, "en").unwrap(),
        "Prague".to_string(),
      )
      .unwrap();

    lirs_storage
      .insert(
        CacheKey::try_new(51.5074, -0.1278, "en").unwrap(),
        "London".to_string(),
      )
      .unwrap();

    assert_eq!(lirs_storage.first().unwrap().1, "London");
    assert_eq!(lirs_storage.last().unwrap().1, "Bratislava");

    lirs_storage.get(&CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap());

    assert_eq!(lirs_storage.first().unwrap().1, "Bratislava");
    assert_eq!(lirs_storage.last().unwrap().1, "Prague");
  }

  #[test]
  fn lirs_serialization_compatible_with_deque() {
    use crate::DequeStorage;

    let (mut storage, _tmp) = create_test_storage();
    let mut deque = DequeStorage::default();
    deque.memory_max_size(100_000);

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

    deque.flush(&mut storage).unwrap();
    drop(deque);

    let mut lirs = LirsStorage::default();
    lirs.memory_max_size(100_000);
    lirs.read(&mut storage).unwrap();

    assert_eq!(lirs.in_memory_record_count(), 2);
    assert_eq!(
      lirs.get(&CacheKey::try_new(48.1645819, 17.1847104, "en").unwrap()),
      Some(&"Bratislava, Slovakia".to_string())
    );
    assert_eq!(
      lirs.get(&CacheKey::try_new(50.073658, 14.418540, "en").unwrap()),
      Some(&"Prague, Czechia".to_string())
    );

    lirs.flush(&mut storage).unwrap();
    drop(lirs);

    let mut deque = DequeStorage::default();
    deque.memory_max_size(100_000);
    deque.read(&mut storage).unwrap();

    assert_eq!(deque.in_memory_record_count(), 2);
  }
}
