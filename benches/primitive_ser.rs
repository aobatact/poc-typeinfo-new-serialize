use criterion::{Criterion, black_box, criterion_group, criterion_main};
use poc_typeinfo_new_deser::Ser;
use poc_typeinfo_new_deser::json::JsonSerializer;

fn bench_u32(c: &mut Criterion) {
    c.bench_function("u32", |b| {
        let mut buf = Vec::with_capacity(64);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box(42_u32).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_i64(c: &mut Criterion) {
    c.bench_function("i64", |b| {
        let mut buf = Vec::with_capacity(64);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box(-123456_i64).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_f64(c: &mut Criterion) {
    c.bench_function("f64", |b| {
        let mut buf = Vec::with_capacity(64);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box(3.14159_f64).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_bool(c: &mut Criterion) {
    c.bench_function("bool", |b| {
        let mut buf = Vec::with_capacity(64);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box(true).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_u8(c: &mut Criterion) {
    c.bench_function("u8", |b| {
        let mut buf = Vec::with_capacity(64);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box(255_u8).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_struct_with_primitives(c: &mut Criterion) {
    struct Point {
        #[allow(unused)]
        x: f64,
        #[allow(unused)]
        y: f64,
        #[allow(unused)]
        z: f64,
    }

    c.bench_function("struct_3xf64", |b| {
        let mut buf = Vec::with_capacity(256);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            let p = Point {
                x: black_box(1.0),
                y: black_box(2.0),
                z: black_box(3.0),
            };
            p.serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_array_u32(c: &mut Criterion) {
    c.bench_function("array_10xu32", |b| {
        let mut buf = Vec::with_capacity(256);
        b.iter(|| {
            buf.clear();
            let mut json = JsonSerializer::new(&mut buf);
            black_box([1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10]).serialize(&mut json).unwrap();
            black_box(&buf);
        });
    });
}

criterion_group!(
    benches,
    bench_u32,
    bench_i64,
    bench_f64,
    bench_bool,
    bench_u8,
    bench_struct_with_primitives,
    bench_array_u32,
);
criterion_main!(benches);
