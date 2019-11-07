#[macro_use]
extern crate criterion;

use criterion::Criterion;
// TODO: figure out what this is for
use criterion::black_box;
use rand::Rng;

use tilemap::tile_world::{
    TileMap, GridCoord, TileValue
};


fn criterion_benchmark(c: &mut Criterion) {
    let mut world = TileMap::new();
    let mut rng = rand::thread_rng();

    c.bench_function("map_read_repeated", |b| b.iter(|| world.sample(&GridCoord{x: black_box(0), y: black_box(0)})));
    c.bench_function("map_write_repeated", |b| b.iter(|| world.make_change(&GridCoord{x: black_box(0), y: black_box(0)}, &TileValue::Error)));
    c.bench_function("read_write_read_repeated", |b| b.iter(|| {
        let coord = GridCoord{x: rng.gen::<i64>(), y: rng.gen::<i64>()};
        world.sample(&coord);
        world.make_change(&coord, &TileValue::Error);
        world.sample(&coord);
    }));

    c.bench_function("dense_map_read_random", |b| b.iter(|| world.sample(&GridCoord{x: rng.gen::<i64>() % 16, y: rng.gen::<i64>() % 16})));
    c.bench_function("dense_map_write_random", |b| b.iter(|| world.make_change(&GridCoord{x: rng.gen::<i64>() % 16, y: rng.gen::<i64>() % 16}, &TileValue::Error)));
    c.bench_function("dense_map_read_write_read_random", |b| b.iter(|| {
        let coord = GridCoord{x: rng.gen::<i64>() % 16, y: rng.gen::<i64>() % 16};
        world.sample(&coord);
        world.make_change(&coord, &TileValue::Error);
        world.sample(&coord);
    }));

    c.bench_function("sparse_map_read_random", |b| b.iter(|| world.sample(&GridCoord{x: rng.gen::<i64>(), y: rng.gen::<i64>()})));
    c.bench_function("sparse_map_write_random", |b| b.iter(|| world.make_change(&GridCoord{x: rng.gen::<i64>(), y: rng.gen::<i64>()}, &TileValue::Error)));
    c.bench_function("sparse_map_read_write_read_random", |b| b.iter(|| {
        let coord = GridCoord{x: rng.gen::<i64>(), y: rng.gen::<i64>()};
        world.sample(&coord);
        world.make_change(&coord, &TileValue::Error);
        world.sample(&coord);
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);