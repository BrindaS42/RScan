use crate::data::{Employee, EmployeeTable};
use crate::operators::{eval_aggregate, eval_filter, eval_group_by};
use crate::query::{AggregateFunc, Column, Query, SelectExpr};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl QueryResult {
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self { headers, rows }
    }

    /// Normalizes row order for strict result equivalence testing between sequential and parallel engines.
    pub fn normalized_rows(&self) -> Vec<Vec<String>> {
        let mut sorted = self.rows.clone();
        sorted.sort();
        sorted
    }
}

impl PartialEq for QueryResult {
    fn eq(&self, other: &Self) -> bool {
        self.headers == other.headers && self.normalized_rows() == other.normalized_rows()
    }
}

impl fmt::Display for QueryResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.headers.is_empty() {
            return write!(f, "(Empty result)");
        }

        // Compute column widths
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Format header
        let header_line: Vec<String> = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:width$}", h, width = widths[i]))
            .collect();
        writeln!(f, "{}", header_line.join(" | "))?;

        // Format separator
        let sep_line: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
        writeln!(f, "{}", sep_line.join("-+-"))?;

        // Format rows
        for row in &self.rows {
            let row_line: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{:width$}", cell, width = widths[i]))
                .collect();
            writeln!(f, "{}", row_line.join(" | "))?;
        }

        writeln!(f, "\n({} rows returned)", self.rows.len())
    }
}

/// Sequentially executes a parsed Query AST against a columnar `EmployeeTable`.
pub fn execute_sequential(table: &EmployeeTable, query: &Query) -> Result<QueryResult, String> {
    let len = table.len();
    let initial_indices: Vec<usize> = if let Some(ref filter) = query.filter {
        eval_filter(table, filter)
    } else {
        (0..len).collect()
    };

    build_result_from_indices(table, query, &initial_indices)
}

/// Parallely executes a parsed Query AST against a columnar `EmployeeTable` using Rayon.
pub fn execute_parallel(table: &EmployeeTable, query: &Query) -> Result<QueryResult, String> {
    let len = table.len();
    if len < 50_000 {
        // Fallback to sequential for very small tables to avoid scheduling overhead
        return execute_sequential(table, query);
    }

    // Partition row indices into parallel chunks
    let chunk_size = 50_000;
    let chunk_indices: Vec<Vec<usize>> = (0..len)
        .collect::<Vec<usize>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Parallel filtering phase
    let filtered_chunks: Vec<Vec<usize>> = chunk_indices
        .par_iter()
        .map(|chunk| {
            if let Some(ref filter) = query.filter {
                // Filter sub-chunk locally
                let sub_table = slice_table(table, chunk);
                let local_matches = eval_filter(&sub_table, filter);
                local_matches.into_iter().map(|idx| chunk[idx]).collect()
            } else {
                chunk.clone()
            }
        })
        .collect();

    // Flatten matching indices
    let initial_indices: Vec<usize> = filtered_chunks.into_iter().flatten().collect();

    build_result_from_indices(table, query, &initial_indices)
}

/// Lightweight row-oriented baseline execution for architectural comparison.
pub fn execute_row_baseline(employees: &[Employee], query: &Query) -> Result<QueryResult, String> {
    let table = EmployeeTable::from_employees(employees);
    execute_sequential(&table, query)
}

fn slice_table(table: &EmployeeTable, indices: &[usize]) -> EmployeeTable {
    let mut sub = EmployeeTable::with_capacity(indices.len());
    for &idx in indices {
        sub.ids.push(table.ids[idx]);
        sub.ages.push(table.ages[idx]);
        sub.departments.push(table.departments[idx]);
        sub.salaries.push(table.salaries[idx]);
        sub.experience.push(table.experience[idx]);
    }
    sub
}

fn build_result_from_indices(
    table: &EmployeeTable,
    query: &Query,
    indices: &[usize],
) -> Result<QueryResult, String> {
    let mut headers = Vec::new();
    for expr in &query.select {
        headers.push(expr.to_string());
    }

    let mut rows = Vec::new();

    if let Some(group_col) = query.group_by {
        let groups = eval_group_by(table, indices, group_col);

        for (group_val, group_indices) in groups {
            let mut row = Vec::new();
            for expr in &query.select {
                match expr {
                    SelectExpr::Col(c) if *c == group_col => {
                        row.push(group_val.clone());
                    }
                    SelectExpr::Col(c) => {
                        return Err(format!(
                            "Column '{}' must appear in GROUP BY clause or be aggregated",
                            c
                        ));
                    }
                    SelectExpr::Agg(agg) => {
                        let val = eval_aggregate(table, &group_indices, agg);
                        row.push(format_agg_value(agg, val));
                    }
                    SelectExpr::Star => {
                        return Err("Cannot SELECT * with GROUP BY".to_string());
                    }
                }
            }
            rows.push(row);
        }
    } else {
        // No GROUP BY
        let has_agg = query.select.iter().any(|e| matches!(e, SelectExpr::Agg(_)));
        if has_agg {
            let mut row = Vec::new();
            for expr in &query.select {
                match expr {
                    SelectExpr::Agg(agg) => {
                        let val = eval_aggregate(table, indices, agg);
                        row.push(format_agg_value(agg, val));
                    }
                    _ => {
                        return Err(
                            "Cannot combine non-aggregate columns without GROUP BY".to_string()
                        );
                    }
                }
            }
            rows.push(row);
        } else {
            // Raw projection of rows
            let limit = 100; // Cap displayed rows at 100
            for &idx in indices.iter().take(limit) {
                let mut row = Vec::new();
                for expr in &query.select {
                    match expr {
                        SelectExpr::Star => {
                            row.push(table.ids[idx].to_string());
                            row.push(table.ages[idx].to_string());
                            row.push(table.departments[idx].to_string());
                            row.push(format!("{:.2}", table.salaries[idx]));
                            row.push(table.experience[idx].to_string());
                        }
                        SelectExpr::Col(Column::Id) => row.push(table.ids[idx].to_string()),
                        SelectExpr::Col(Column::Age) => row.push(table.ages[idx].to_string()),
                        SelectExpr::Col(Column::Department) => {
                            row.push(table.departments[idx].to_string())
                        }
                        SelectExpr::Col(Column::Salary) => {
                            row.push(format!("{:.2}", table.salaries[idx]))
                        }
                        SelectExpr::Col(Column::Experience) => {
                            row.push(table.experience[idx].to_string())
                        }
                        SelectExpr::Agg(_) => unreachable!(),
                    }
                }
                rows.push(row);
            }

            if query.select.contains(&SelectExpr::Star) {
                headers = vec![
                    "id".to_string(),
                    "age".to_string(),
                    "department".to_string(),
                    "salary".to_string(),
                    "experience".to_string(),
                ];
            }
        }
    }

    Ok(QueryResult::new(headers, rows))
}

fn format_agg_value(agg: &AggregateFunc, val: f64) -> String {
    match agg {
        AggregateFunc::CountStar => format!("{}", val as u64),
        AggregateFunc::Sum(_) => format!("{:.2}", val),
        AggregateFunc::Avg(_) => format!("{:.2}", val),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Department;
    use crate::parser::parse_query;

    fn sample_table() -> EmployeeTable {
        let mut table = EmployeeTable::new();
        table.push(Employee {
            id: 1,
            age: 25,
            department: Department::Engineering,
            salary: 110000.0,
            experience: 3,
        });
        table.push(Employee {
            id: 2,
            age: 35,
            department: Department::Engineering,
            salary: 140000.0,
            experience: 8,
        });
        table.push(Employee {
            id: 3,
            age: 40,
            department: Department::Sales,
            salary: 90000.0,
            experience: 10,
        });
        table.push(Employee {
            id: 4,
            age: 28,
            department: Department::Sales,
            salary: 70000.0,
            experience: 4,
        });
        table.push(Employee {
            id: 5,
            age: 50,
            department: Department::HR,
            salary: 80000.0,
            experience: 20,
        });
        table
    }

    #[test]
    fn test_sequential_and_parallel_equivalence() {
        let table = sample_table();

        let queries = vec![
            "SELECT department, SUM(salary) WHERE salary > 75000 GROUP BY department",
            "SELECT department, COUNT(*) WHERE salary > 75000 GROUP BY department",
            "SELECT department, AVG(salary) WHERE age >= 30 GROUP BY department",
        ];

        for q_str in queries {
            let q = parse_query(q_str).unwrap();
            let seq = execute_sequential(&table, &q).unwrap();
            let par = execute_parallel(&table, &q).unwrap();
            let row = execute_row_baseline(&table.to_employees(), &q).unwrap();

            assert_eq!(
                seq, par,
                "Sequential vs Parallel mismatch on query: {}",
                q_str
            );
            assert_eq!(
                seq, row,
                "Sequential vs Row-baseline mismatch on query: {}",
                q_str
            );
        }
    }
}
