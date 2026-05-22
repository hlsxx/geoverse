# geoverse

A Rust library for caching reverse geocode results. Single crate, no workspace.

## Quick commands

```sh
cargo test                              # unit + integration tests (uses "testing" feature via default)
cargo test --features "dhat-heap,testing" --test allocations -- --test-threads=1  # allocation tests
cargo bench                             # criterion benchmarks (benches/cache.rs)
cargo fmt                               # formatter; indent is 2 spaces (rustfmt.toml)
cargo clippy                            # lints
```

## Architecture

- **`GeoCache<S: StorageStrategy>`** — generic cache. Type parameter selects eviction strategy.
- **Storage strategies**: `DequeStorage` (FIFO, the only implemented one), `lru.rs` is an empty placeholder — don't use `LruStorage`.
- **Persistence**: optional file-backed storage. Must call `.init()` after `GeoCache::new()` for disk to work.
- **`CacheKey`**: 20-byte fixed-width encoding: `[lang(2) ';' lat_microdeg(8) ';' lng_microdeg(8)]`. Coordinates stored as i32 microdegrees (1° = 1_000_000 µ°).
- **Flush strategies**: `Never` (default, in-memory only), `Immediately`, `RecordCount(n)`.

## Non-obvious facts

- **Edition 2024** — requires Rust 1.85+.
- **`default = ["testing"]`** — the `testing` feature is always on by default. It gates `with_capacity()` and `StorageStrategyWithCapacity`.
- **`dhat-heap` + `testing`** both required for allocation tests. Must run single-threaded (`--test-threads=1`).
- **`#[must_use]` on `init()`** — easy to miss. Without calling `init()`, disk persistence won't load data.
- **VSCode** (`rust-analyzer.cargo.features`): pre-configured to enable `testing` feature.

## Public API (re-exported from `lib.rs`)

```rust
use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder, StorageFlushStrategy};
use geoverse::{convert_coords_into_microdeg, convert_lang_to_u16, convert_u16_to_lang};
```
