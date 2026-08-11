use clap::{Parser, Subcommand};
use rscan::{
    execute_parallel, execute_sequential, parse_query, scan_dir, search_parallel,
    search_sequential, EmployeeTable,
};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "rscan",
    author,
    version,
    about = "RScan — Fast Local Analytics & Text Search Engine in Rust",
    long_about = "RScan is a dual-engine CLI tool supporting both recursive text pattern search across files and SQL-like analytical query processing over local columnar CSV datasets using sequential or Rayon parallel execution."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Search files recursively for a text pattern
    Search {
        /// Directory path to search
        directory: PathBuf,

        /// Text pattern to search for
        pattern: String,

        /// Optional file extension filter (e.g. rs or txt)
        #[arg(short, long)]
        ext: Option<String>,

        /// Use parallel execution powered by Rayon across files
        #[arg(short, long)]
        parallel: bool,
    },

    /// Execute a SQL-like analytical query over CSV data in a directory
    Query {
        /// Target directory containing CSV file(s)
        directory: PathBuf,

        /// SQL-like query string (e.g. "SELECT department, SUM(salary) WHERE salary > 100000 GROUP BY department")
        query: String,

        /// Use parallel execution powered by Rayon across columnar dataset chunks
        #[arg(short, long)]
        parallel: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            directory,
            pattern,
            ext,
            parallel,
        } => {
            if !directory.exists() || !directory.is_dir() {
                eprintln!(
                    "Error: Directory '{}' does not exist or is not a directory.",
                    directory.display()
                );
                process::exit(1);
            }

            let start_time = Instant::now();
            let files = scan_dir(&directory, ext.as_deref());

            let mode_str = if parallel { "parallel" } else { "sequential" };
            let matches = if parallel {
                search_parallel(&files, &pattern)
            } else {
                search_sequential(&files, &pattern)
            };

            let elapsed = start_time.elapsed();

            println!("RScan Search Engine\n");
            println!("Directory : {}", directory.display());
            println!("Pattern   : {}", pattern);
            println!("Mode      : {}", mode_str);
            println!("Files     : {}", files.len());
            println!("Matches   : {}", matches.len());
            println!("Time      : {:.2?}\n", elapsed);

            let limit = 100;
            for m in matches.iter().take(limit) {
                println!("{}:{}: {}", m.path.display(), m.line_number, m.line_content);
            }

            if matches.len() > limit {
                println!("\n... and {} more matches omitted.", matches.len() - limit);
            }
        }

        Commands::Query {
            directory,
            query,
            parallel,
        } => {
            if !directory.exists() || !directory.is_dir() {
                eprintln!(
                    "Error: Directory '{}' does not exist or is not a directory.",
                    directory.display()
                );
                process::exit(1);
            }

            // 1. Parse SQL-like query
            let parsed_query = match parse_query(&query) {
                Ok(q) => q,
                Err(err) => {
                    eprintln!("Query Parser Error: {}", err);
                    process::exit(1);
                }
            };

            // 2. Discover and load CSV into columnar EmployeeTable
            let start_load = Instant::now();
            let table = match EmployeeTable::from_csv_dir(&directory) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("Data Engine Error: {}", err);
                    process::exit(1);
                }
            };
            let load_elapsed = start_load.elapsed();

            // 3. Execute query
            let mode_str = if parallel { "parallel" } else { "sequential" };
            let start_exec = Instant::now();
            let result = if parallel {
                execute_parallel(&table, &parsed_query)
            } else {
                execute_sequential(&table, &parsed_query)
            };
            let exec_elapsed = start_exec.elapsed();

            match result {
                Ok(res) => {
                    println!("RScan Analytical Query Engine\n");
                    println!("Directory : {}", directory.display());
                    println!("Query     : {}", parsed_query);
                    println!("Mode      : {}", mode_str);
                    println!("Dataset   : {} rows", table.len());
                    println!("Load Time : {:.2?}", load_elapsed);
                    println!("Exec Time : {:.2?}\n", exec_elapsed);
                    println!("{}", res);
                }
                Err(err) => {
                    eprintln!("Query Execution Error: {}", err);
                    process::exit(1);
                }
            }
        }
    }
}
