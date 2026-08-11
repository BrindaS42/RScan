use crate::data::Department;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    Id,
    Age,
    Department,
    Salary,
    Experience,
}

impl Column {
    pub fn as_str(&self) -> &'static str {
        match self {
            Column::Id => "id",
            Column::Age => "age",
            Column::Department => "department",
            Column::Salary => "salary",
            Column::Experience => "experience",
        }
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Column {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "id" => Ok(Column::Id),
            "age" => Ok(Column::Age),
            "department" | "dept" => Ok(Column::Department),
            "salary" => Ok(Column::Salary),
            "experience" | "years_experience" => Ok(Column::Experience),
            _ => Err(format!("Unknown column: '{}'", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Gt,  // >
    Lt,  // <
    Gte, // >=
    Lte, // <=
    Eq,  // == or =
    Neq, // !=
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            Operator::Gt => ">",
            Operator::Lt => "<",
            Operator::Gte => ">=",
            Operator::Lte => "<=",
            Operator::Eq => "==",
            Operator::Neq => "!=",
        };
        write!(f, "{}", op_str)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Dept(Department),
    Str(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Dept(d) => write!(f, "{}", d),
            Value::Str(s) => write!(f, "'{}'", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunc {
    Sum(Column),
    CountStar,
    Avg(Column),
}

impl fmt::Display for AggregateFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateFunc::Sum(col) => write!(f, "SUM({})", col),
            AggregateFunc::CountStar => write!(f, "COUNT(*)"),
            AggregateFunc::Avg(col) => write!(f, "AVG({})", col),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectExpr {
    Star,
    Col(Column),
    Agg(AggregateFunc),
}

impl fmt::Display for SelectExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectExpr::Star => write!(f, "*"),
            SelectExpr::Col(col) => write!(f, "{}", col),
            SelectExpr::Agg(agg) => write!(f, "{}", agg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub column: Column,
    pub operator: Operator,
    pub literal: Value,
}

impl fmt::Display for FilterCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.column, self.operator, self.literal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub select: Vec<SelectExpr>,
    pub filter: Option<FilterCondition>,
    pub group_by: Option<Column>,
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let select_str: Vec<String> = self.select.iter().map(|s| s.to_string()).collect();
        write!(f, "SELECT {}", select_str.join(", "))?;
        if let Some(ref filter) = self.filter {
            write!(f, " WHERE {}", filter)?;
        }
        if let Some(group_by) = self.group_by {
            write!(f, " GROUP BY {}", group_by)?;
        }
        Ok(())
    }
}
