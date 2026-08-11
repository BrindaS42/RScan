use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rscan::data::{Department, Employee};
use rscan::{
    execute_parallel, execute_row_baseline, execute_sequential, parse_query, EmployeeTable,
};

fn generate_synthetic_table(num_rows: usize) -> (EmployeeTable, Vec<Employee>) {
    let departments = Department::ALL;
    let mut table = EmployeeTable::with_capacity(num_rows);
    let mut rows = Vec::with_capacity(num_rows);

    let mut state: u64 = 42;
    let mut next_rand = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state >> 32
    };

    for i in 1..=num_rows {
        let r1 = next_rand();
        let r2 = next_rand();
        let r3 = next_rand();

        let id = i as u64;
        let age = 22 + (r1 % 43) as u32;
        let dept = departments[(r2 % 5) as usize];
        let salary = 40000.0 + ((r3 % 120000) as f64);
        let experience = (age - 22) / 2;

        let emp = Employee {
            id,
            age,
            department: dept,
            salary,
            experience,
        };

        table.push(emp.clone());
        rows.push(emp);
    }

    (table, rows)
}

fn bench_query_execution(c: &mut Criterion) {
    let row_count = 1_000_000;
    println!("Pre-allocating 1,000,000 synthetic employee records for benchmark...");
    let (table, rows) = generate_synthetic_table(row_count);

    let query_str = "SELECT department, SUM(salary) WHERE salary > 100000 GROUP BY department";
    let query = parse_query(query_str).unwrap();

    let mut group = c.benchmark_group("Analytical Query Execution (1M Rows)");
    group.sample_size(10); // Reasonable benchmark iteration time

    // 1. Columnar Sequential Execution
    group.bench_with_input(
        BenchmarkId::new("columnar_sequential", row_count),
        &table,
        |b, table| {
            b.iter(|| {
                let res = execute_sequential(black_box(table), black_box(&query)).unwrap();
                black_box(res);
            });
        },
    );

    // 2. Columnar Parallel Execution (Rayon)
    group.bench_with_input(
        BenchmarkId::new("columnar_parallel", row_count),
        &table,
        |b, table| {
            b.iter(|| {
                let res = execute_parallel(black_box(table), black_box(&query)).unwrap();
                black_box(res);
            });
        },
    );

    // 3. Row-Oriented Baseline Execution
    group.bench_with_input(
        BenchmarkId::new("row_baseline_sequential", row_count),
        &rows,
        |b, rows| {
            b.iter(|| {
                let res = execute_row_baseline(black_box(rows), black_box(&query)).unwrap();
                black_box(res);
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_query_execution);
criterion_main!(benches);
