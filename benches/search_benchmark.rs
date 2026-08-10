use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rscan::{search_parallel, search_sequential};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn setup_benchmark_files(num_files: usize) -> (TempDir, Vec<PathBuf>) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory for benchmark");
    let dir_path = temp_dir.path();
    let mut files = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let file_path = dir_path.join(format!("file_{}.txt", i));
        let mut file = File::create(&file_path).expect("Failed to create synthetic test file");
        if i % 2 == 0 {
            writeln!(
                file,
                "Line 1: standard file content\nLine 2: TODO fix performance issue\nLine 3: end"
            )
            .expect("Failed to write to benchmark file");
        } else {
            writeln!(
                file,
                "Line 1: quick brown fox\nLine 2: jumps over lazy dog\nLine 3: end"
            )
            .expect("Failed to write to benchmark file");
        }
        files.push(file_path);
    }

    (temp_dir, files)
}

fn bench_search(c: &mut Criterion) {
    let file_count = 5000;
    let (_temp_dir, files) = setup_benchmark_files(file_count);
    let pattern = "TODO";

    let mut group = c.benchmark_group("RScan Search");
    group.sample_size(10); // Keep benchmark duration reasonable on laptops

    group.bench_with_input(
        BenchmarkId::new("sequential", file_count),
        &files,
        |b, files| {
            b.iter(|| search_sequential(black_box(files), black_box(pattern)));
        },
    );

    group.bench_with_input(
        BenchmarkId::new("parallel", file_count),
        &files,
        |b, files| {
            b.iter(|| search_parallel(black_box(files), black_box(pattern)));
        },
    );

    group.finish();
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
