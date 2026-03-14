use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use geoverse::{DequeStorage, GeoCache, GeoCacheConfigBuilder};

// Used in storage strategy internal data structures
const CAPACITY_SIZE: usize = 100;

fn create_example_deque_geo_cache() -> GeoCache<DequeStorage> {
  GeoCache::with_capacity(GeoCacheConfigBuilder::default().build(), CAPACITY_SIZE)
}

fn bench_insert(c: &mut Criterion) {
  let mut group = c.benchmark_group("insert");

  for size in [100, 1_000, 10_000] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
      b.iter_batched(
        || {
          (0..size)
            .map(|i| (48.0 + (i as f64 * 0.0001), format!("Address {i}")))
            .collect::<Vec<_>>()
        },
        |data| {
          let mut geo_cache = create_example_deque_geo_cache();
          for (lat, addr) in data {
            geo_cache
              .insert(std::hint::black_box((lat, 17.1847104, "sk")), addr)
              .unwrap();
          }
        },
        criterion::BatchSize::SmallInput,
      );
    });
  }
  group.finish();
}

fn bench_get(c: &mut Criterion) {
  let mut group = c.benchmark_group("get");

  for size in [100, 1_000, 10_000] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
      let mut geo_cache = create_example_deque_geo_cache();
      for i in 0..size {
        let lat = 48.0 + (i as f64 * 0.0001);
        geo_cache
          .insert((lat, 17.1847104, "sk"), format!("Address {i}"))
          .unwrap();
      }

      b.iter(|| {
        for i in 0..size {
          let lat = 48.0 + (i as f64 * 0.0001);
          geo_cache
            .get(std::hint::black_box((lat, 17.1847104, "sk")))
            .unwrap();
        }
      });
    });
  }
  group.finish();
}

criterion_group!(benches, bench_insert, bench_get);
criterion_main!(benches);
