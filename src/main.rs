use clap::Parser;
use rscan::{scan_dir, search_parallel, search_sequential};
use std::path::PathBuf;
use std::process;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(
    name = "rscan",
    author,
    version,
    about = "A fast recursive file-search CLI written in Rust",
    long_about = None
)]
struct Args {
    /// Directory to search
    directory: PathBuf,

    /// Text pattern to search for
    pattern: String,

    /// Optional file extension filter (e.g. rs or txt)
    #[arg(short, long)]
    ext: Option<String>,

    /// Use parallel search powered by Rayon
    #[arg(short, long)]
    parallel: bool,
}

fn main() {
    let args = Args::parse();

    if !args.directory.exists() || !args.directory.is_dir() {
        eprintln!(
            "Error: Directory '{}' does not exist or is not a directory.",
            args.directory.display()
        );
        process::exit(1);
    }

    let start_time = Instant::now();
    let files = scan_dir(&args.directory, args.ext.as_deref());

    let mode_str = if args.parallel {
        "parallel"
    } else {
        "sequential"
    };
    let matches = if args.parallel {
        search_parallel(&files, &args.pattern)
    } else {
        search_sequential(&files, &args.pattern)
    };

    let elapsed = start_time.elapsed();

    println!("RScan\n");
    println!("Directory : {}", args.directory.display());
    println!("Pattern   : {}", args.pattern);
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
