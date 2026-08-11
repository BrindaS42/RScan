use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use crate::scanner::scan_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Department {
    Engineering,
    Sales,
    HR,
    Finance,
    Marketing,
}

impl Department {
    pub const ALL: [Department; 5] = [
        Department::Engineering,
        Department::Sales,
        Department::HR,
        Department::Finance,
        Department::Marketing,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Department::Engineering => "Engineering",
            Department::Sales => "Sales",
            Department::HR => "HR",
            Department::Finance => "Finance",
            Department::Marketing => "Marketing",
        }
    }
}

impl fmt::Display for Department {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Department {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "engineering" => Ok(Department::Engineering),
            "sales" => Ok(Department::Sales),
            "hr" => Ok(Department::HR),
            "finance" => Ok(Department::Finance),
            "marketing" => Ok(Department::Marketing),
            _ => Err(format!("Unknown department: '{}'", s)),
        }
    }
}

/// Row-oriented Employee struct used as a lightweight baseline comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Employee {
    pub id: u64,
    pub age: u32,
    pub department: Department,
    pub salary: f64,
    pub experience: u32,
}

/// Column-oriented representation of employee data.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EmployeeTable {
    pub ids: Vec<u64>,
    pub ages: Vec<u32>,
    pub departments: Vec<Department>,
    pub salaries: Vec<f64>,
    pub experience: Vec<u32>,
}

impl EmployeeTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            ids: Vec::with_capacity(capacity),
            ages: Vec::with_capacity(capacity),
            departments: Vec::with_capacity(capacity),
            salaries: Vec::with_capacity(capacity),
            experience: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, emp: Employee) {
        self.ids.push(emp.id);
        self.ages.push(emp.age);
        self.departments.push(emp.department);
        self.salaries.push(emp.salary);
        self.experience.push(emp.experience);
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn from_employees(employees: &[Employee]) -> Self {
        let mut table = Self::with_capacity(employees.len());
        for emp in employees {
            table.push(emp.clone());
        }
        table
    }

    pub fn to_employees(&self) -> Vec<Employee> {
        let mut rows = Vec::with_capacity(self.len());
        for i in 0..self.len() {
            rows.push(Employee {
                id: self.ids[i],
                age: self.ages[i],
                department: self.departments[i],
                salary: self.salaries[i],
                experience: self.experience[i],
            });
        }
        rows
    }

    /// Loads a CSV file into a columnar `EmployeeTable`.
    pub fn from_csv_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("Failed to open file '{}': {}", path.as_ref().display(), e))?;
        let reader = BufReader::new(file);

        let mut table = Self::new();
        let mut lines = reader.lines();

        // Skip header if present
        if let Some(first_line) = lines.next() {
            let header = first_line.map_err(|e| format!("Error reading header: {}", e))?;
            if !header.to_lowercase().starts_with("id,") {
                // Not a header, parse first row
                parse_row(&header, &mut table)?;
            }
        }

        for (line_num, line_res) in lines.enumerate() {
            let line =
                line_res.map_err(|e| format!("Error reading line {}: {}", line_num + 2, e))?;
            if line.trim().is_empty() {
                continue;
            }
            parse_row(&line, &mut table)?;
        }

        Ok(table)
    }

    /// Recursively discovers and loads all CSV files in a target directory.
    pub fn from_csv_dir<P: AsRef<Path>>(dir: P) -> Result<Self, String> {
        let csv_files = scan_dir(dir.as_ref(), Some("csv"));
        if csv_files.is_empty() {
            return Err(format!(
                "No CSV files found in directory '{}'",
                dir.as_ref().display()
            ));
        }

        let mut combined_table = Self::new();
        for file_path in csv_files {
            let table = Self::from_csv_file(&file_path)?;
            combined_table.ids.extend(table.ids);
            combined_table.ages.extend(table.ages);
            combined_table.departments.extend(table.departments);
            combined_table.salaries.extend(table.salaries);
            combined_table.experience.extend(table.experience);
        }

        Ok(combined_table)
    }
}

fn parse_row(line: &str, table: &mut EmployeeTable) -> Result<(), String> {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 5 {
        return Err(format!(
            "Malformed CSV row (expected 5 columns): '{}'",
            line
        ));
    }

    let id = parts[0]
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("Invalid id '{}': {}", parts[0], e))?;
    let age = parts[1]
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid age '{}': {}", parts[1], e))?;
    let department = parts[2].trim().parse::<Department>()?;
    let salary = parts[3]
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("Invalid salary '{}': {}", parts[3], e))?;
    let experience = parts[4]
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Invalid experience '{}': {}", parts[4], e))?;

    table.ids.push(id);
    table.ages.push(age);
    table.departments.push(department);
    table.salaries.push(salary);
    table.experience.push(experience);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_csv_loading() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.csv");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            "id,age,department,salary,years_experience\n1,30,Engineering,105000.0,5\n2,45,Sales,85000.0,12"
        )
        .unwrap();

        let table = EmployeeTable::from_csv_file(&file_path).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table.ids, vec![1, 2]);
        assert_eq!(table.ages, vec![30, 45]);
        assert_eq!(
            table.departments,
            vec![Department::Engineering, Department::Sales]
        );
        assert_eq!(table.salaries, vec![105000.0, 85000.0]);
        assert_eq!(table.experience, vec![5, 12]);
    }
}
