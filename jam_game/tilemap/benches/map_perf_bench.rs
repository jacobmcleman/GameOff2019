#[macro_use]
extern crate criterion;

use criterion::Criterion;
// TODO: figure out what this is for
use criterion::black_box;
use rand::Rng;

use tilemap::tile_world::{
    TileMap, GridCoord
};


fn criterion_benchmark(c: &mut Criterion) {
    let mut world = TileMap::new();
    let mut rng = rand::thread_rng();

    c.bench_function("dense_map_access_random", |b| b.iter(|| world.sample(&GridCoord{x: rng.gen::<i64>() % 16, y: rng.gen::<i64>() % 16})));
    c.bench_function("sparse_map_access_random", |b| b.iter(|| world.sample(&GridCoord{x: black_box(0), y: black_box(0)})));
    c.bench_function("map_access_repeated", |b| b.iter(|| world.sample(&GridCoord{x: rng.gen::<i64>() % 16, y: rng.gen::<i64>() % 16})));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);