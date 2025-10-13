use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use floatctl_core::{
    stream::{RawValueStream, ConvStream},
    Conversation,
};
use std::path::PathBuf;

fn get_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("fixtures")
        .join(name)
}

fn bench_raw_value_stream(c: &mut Criterion) {
    let fixture = get_fixture_path("small.json");

    c.bench_function("RawValueStream::parse_small_array", |b| {
        b.iter(|| {
            let stream = RawValueStream::from_path(&fixture).unwrap();
            let mut count = 0;
            for result in stream {
                let _value = result.unwrap();
                count += 1;
            }
            black_box(count)
        });
    });
}

fn bench_conv_stream(c: &mut Criterion) {
    let fixture = get_fixture_path("small.json");

    c.bench_function("ConvStream::parse_small_array", |b| {
        b.iter(|| {
            let stream = ConvStream::from_path(&fixture).unwrap();
            let mut count = 0;
            for result in stream {
                let _conv = result.unwrap();
                count += 1;
            }
            black_box(count)
        });
    });
}

fn bench_conversation_parse(c: &mut Criterion) {
    // Load one conversation for parsing benchmark
    let fixture = get_fixture_path("small.json");
    let stream = RawValueStream::from_path(&fixture).unwrap();
    let value = stream.into_iter().next().unwrap().unwrap();

    c.bench_function("Conversation::from_export", |b| {
        b.iter(|| {
            let conv = Conversation::from_export(black_box(value.clone())).unwrap();
            black_box(conv)
        });
    });
}

fn bench_stream_comparison(c: &mut Criterion) {
    let fixture = get_fixture_path("small.json");
    let mut group = c.benchmark_group("stream_comparison");

    group.bench_with_input(
        BenchmarkId::new("RawValueStream", "small"),
        &fixture,
        |b, path| {
            b.iter(|| {
                let stream = RawValueStream::from_path(path).unwrap();
                let count = stream.count();
                black_box(count)
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("ConvStream", "small"),
        &fixture,
        |b, path| {
            b.iter(|| {
                let stream = ConvStream::from_path(path).unwrap();
                let count = stream.count();
                black_box(count)
            });
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_raw_value_stream,
    bench_conv_stream,
    bench_conversation_parse,
    bench_stream_comparison,
);

criterion_main!(benches);
