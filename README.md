# 🌍 Geoverse

A simple, fast reverse geocode cache library.

## Features

- **In-memory cache** — fast, zero setup
- **Disk persistence** — survives process restarts with a file-based backend

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
geoverse = "0.0.1"
```

## Quick Start

### Just in-memory cache

```rust
use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder};

fn main() {
  let config = GeoCacheConfigBuilder::default().build();
  let mut geo_cache = GeoCache::<DequeStorage>::new(config);

  geo_cache
    .insert(
      (48.1645819, 17.1847104, "sk"),
      "Bratislava, Slovakia".to_string(),
    )
    .expect("failed to insert address");

  let address = geo_cache
    .get((48.1645819, 17.1847104, "sk"))
    .expect("error while loading address")
    .expect("address not found");

  println!("{address}");
}
```

### Disk persistence

```rust
use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder, StorageFlushStrategy};

fn main() {
  let config = GeoCacheConfigBuilder::default()
    .storage_file_path("geoverse.bin")
    .storage_flush_strategy(StorageFlushStrategy::Immediately)
    .build();

  let mut geo_cache = GeoCache::<DequeStorage>::new(config);

  geo_cache
    .insert(
      (48.1645819, 17.1847104, "sk"),
      "Bratislava, Slovakia".to_string(),
    )
    .expect("failed to insert address");

  let address = geo_cache
    .get((48.1645819, 17.1847104, "sk"))
    .expect("error while loading address")
    .expect("address not found");

  println!("{address}");
}
```

## Testing

Run the test suite:

```bash
cargo test
```

Run allocation tests:

```bash
cargo test --features "dhat-heap,testing" --test allocations -- --test-threads=1
```

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.
