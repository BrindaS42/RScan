pub mod data;
pub mod executor;
pub mod operators;
pub mod parser;
pub mod query;
pub mod scanner;
pub mod search;

pub use data::{Department, Employee, EmployeeTable};
pub use executor::{execute_parallel, execute_row_baseline, execute_sequential, QueryResult};
pub use parser::parse_query;
pub use query::Query;
pub use scanner::scan_dir;
pub use search::{search_file, search_parallel, search_sequential, SearchResult};
