use rscan::data::Department;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_rows: usize = if args.len() > 1 {
        args[1]
            .parse()
            .unwrap_or_else(|_| panic!("Invalid row count argument: '{}'", args[1]))
    } else {
        1_000_000
    };

    let dir_path = Path::new("data");
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).expect("Failed to create 'data' directory");
    }

    let file_path = dir_path.join("employees.csv");
    println!(
        "Generating {} deterministic employee records at '{}'...",
        num_rows,
        file_path.display()
    );

    let start_time = Instant::now();
    let file = File::create(&file_path).expect("Failed to create 'data/employees.csv'");
    let mut writer = BufWriter::with_capacity(1024 * 1024, file);

    writeln!(writer, "id,age,department,salary,years_experience")
        .expect("Failed to write CSV header");

    let departments = Department::ALL;

    // Linear Congruential Generator (LCG) state for deterministic pseudo-random values
    let mut lcg_state: u64 = 42;
    fn next_rand(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 32
    }

    for i in 1..=num_rows {
        let r1 = next_rand(&mut lcg_state);
        let r2 = next_rand(&mut lcg_state);
        let r3 = next_rand(&mut lcg_state);

        let id = i as u64;
        let age = 22 + (r1 % 43) as u32; // age 22 to 64
        let dept = departments[(r2 % 5) as usize];
        let salary = 40000.0 + ((r3 % 120000) as f64); // salary $40k to $160k
        let experience = (age - 22) / 2;

        writeln!(
            writer,
            "{},{},{},{:.2},{}",
            id, age, dept, salary, experience
        )
        .expect("Failed to write CSV row");
    }

    writer.flush().expect("Failed to flush writer");
    let elapsed = start_time.elapsed();

    println!(
        "Successfully generated {} rows in {:.2?} ({})",
        num_rows,
        elapsed,
        file_path.display()
    );
}
