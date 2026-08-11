use crate::data::{Department, EmployeeTable};
use crate::query::{AggregateFunc, Column, FilterCondition, Operator, Value};
use std::collections::BTreeMap;

/// Returns matching row indices after applying a filter condition to a columnar `EmployeeTable`.
pub fn eval_filter(table: &EmployeeTable, filter: &FilterCondition) -> Vec<usize> {
    let len = table.len();
    let mut matching_indices = Vec::with_capacity(len);

    match filter.column {
        Column::Id => {
            if let Value::Number(val) = filter.literal {
                let target = val as u64;
                for i in 0..len {
                    if compare_num(table.ids[i], target, filter.operator) {
                        matching_indices.push(i);
                    }
                }
            }
        }
        Column::Age => {
            if let Value::Number(val) = filter.literal {
                let target = val as u32;
                for i in 0..len {
                    if compare_num(table.ages[i], target, filter.operator) {
                        matching_indices.push(i);
                    }
                }
            }
        }
        Column::Department => {
            let target_dept = match &filter.literal {
                Value::Dept(d) => Some(*d),
                Value::Str(s) => s.parse::<Department>().ok(),
                _ => None,
            };
            if let Some(target) = target_dept {
                for i in 0..len {
                    let matches = match filter.operator {
                        Operator::Eq => table.departments[i] == target,
                        Operator::Neq => table.departments[i] != target,
                        _ => false,
                    };
                    if matches {
                        matching_indices.push(i);
                    }
                }
            }
        }
        Column::Salary => {
            if let Value::Number(val) = filter.literal {
                for i in 0..len {
                    if compare_f64(table.salaries[i], val, filter.operator) {
                        matching_indices.push(i);
                    }
                }
            }
        }
        Column::Experience => {
            if let Value::Number(val) = filter.literal {
                let target = val as u32;
                for i in 0..len {
                    if compare_num(table.experience[i], target, filter.operator) {
                        matching_indices.push(i);
                    }
                }
            }
        }
    }

    matching_indices
}

/// Groups matching row indices by the specified column (e.g. Department).
pub fn eval_group_by(
    table: &EmployeeTable,
    indices: &[usize],
    group_col: Column,
) -> BTreeMap<String, Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    match group_col {
        Column::Department => {
            for &idx in indices {
                let key = table.departments[idx].to_string();
                groups.entry(key).or_default().push(idx);
            }
        }
        Column::Age => {
            for &idx in indices {
                let key = table.ages[idx].to_string();
                groups.entry(key).or_default().push(idx);
            }
        }
        Column::Id => {
            for &idx in indices {
                let key = table.ids[idx].to_string();
                groups.entry(key).or_default().push(idx);
            }
        }
        Column::Salary => {
            for &idx in indices {
                let key = format!("{:.2}", table.salaries[idx]);
                groups.entry(key).or_default().push(idx);
            }
        }
        Column::Experience => {
            for &idx in indices {
                let key = table.experience[idx].to_string();
                groups.entry(key).or_default().push(idx);
            }
        }
    }

    groups
}

/// Evaluates an aggregate function over a subset of row indices.
pub fn eval_aggregate(table: &EmployeeTable, indices: &[usize], agg: &AggregateFunc) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }

    match agg {
        AggregateFunc::CountStar => indices.len() as f64,
        AggregateFunc::Sum(col) => match col {
            Column::Salary => indices.iter().map(|&i| table.salaries[i]).sum(),
            Column::Age => indices.iter().map(|&i| table.ages[i] as f64).sum(),
            Column::Experience => indices.iter().map(|&i| table.experience[i] as f64).sum(),
            Column::Id => indices.iter().map(|&i| table.ids[i] as f64).sum(),
            Column::Department => 0.0,
        },
        AggregateFunc::Avg(col) => {
            let sum = eval_aggregate(table, indices, &AggregateFunc::Sum(*col));
            sum / (indices.len() as f64)
        }
    }
}

fn compare_num<T: PartialOrd>(a: T, b: T, op: Operator) -> bool {
    match op {
        Operator::Gt => a > b,
        Operator::Lt => a < b,
        Operator::Gte => a >= b,
        Operator::Lte => a <= b,
        Operator::Eq => a == b,
        Operator::Neq => a != b,
    }
}

fn compare_f64(a: f64, b: f64, op: Operator) -> bool {
    match op {
        Operator::Gt => a > b,
        Operator::Lt => a < b,
        Operator::Gte => a >= b,
        Operator::Lte => a <= b,
        Operator::Eq => (a - b).abs() < 1e-6,
        Operator::Neq => (a - b).abs() >= 1e-6,
    }
}
