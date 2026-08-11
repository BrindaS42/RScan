use crate::data::Department;
use crate::query::{AggregateFunc, Column, FilterCondition, Operator, Query, SelectExpr, Value};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Select,
    Where,
    Group,
    By,
    Sum,
    Count,
    Avg,
    Star,
    Comma,
    LParen,
    RParen,
    Operator(Operator),
    Ident(String),
    Number(f64),
    Str(String),
}

pub fn parse_query(sql: &str) -> Result<Query, String> {
    let tokens = tokenize(sql)?;
    if tokens.is_empty() {
        return Err("Query string is empty".to_string());
    }

    let mut pos = 0;
    if tokens.first() != Some(&Token::Select) {
        return Err("Query must start with SELECT".to_string());
    }
    pos += 1; // consume SELECT

    // Parse select list
    let select_exprs = parse_select_list(&tokens, &mut pos)?;

    // Parse optional WHERE clause
    let mut filter = None;
    if pos < tokens.len() && tokens[pos] == Token::Where {
        pos += 1; // consume WHERE
        filter = Some(parse_where_clause(&tokens, &mut pos)?);
    }

    // Parse optional GROUP BY clause
    let mut group_by = None;
    if pos < tokens.len() && tokens[pos] == Token::Group {
        pos += 1; // consume GROUP
        if pos >= tokens.len() || tokens[pos] != Token::By {
            return Err("Expected 'BY' after 'GROUP'".to_string());
        }
        pos += 1; // consume BY

        if pos >= tokens.len() {
            return Err("Expected column name after 'GROUP BY'".to_string());
        }

        if let Token::Ident(ref col_str) = tokens[pos] {
            let col = Column::from_str(col_str)?;
            group_by = Some(col);
            pos += 1;
        } else {
            return Err(format!(
                "Expected group column name, found '{:?}'",
                tokens[pos]
            ));
        }
    }

    if pos < tokens.len() {
        return Err(format!("Unexpected trailing token: '{:?}'", tokens[pos]));
    }

    Ok(Query {
        select: select_exprs,
        filter,
        group_by,
    })
}

fn tokenize(sql: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '>' => {
                if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Operator::Gte));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Gt));
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Operator::Lte));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Lt));
                    i += 1;
                }
            }
            '=' => {
                if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Operator::Eq));
                    i += 2;
                } else {
                    tokens.push(Token::Operator(Operator::Eq));
                    i += 1;
                }
            }
            '!' => {
                if i + 1 < len && chars[i + 1] == '=' {
                    tokens.push(Token::Operator(Operator::Neq));
                    i += 2;
                } else {
                    return Err("Unexpected character '!' (expected '!=')".to_string());
                }
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < len && chars[i] != quote {
                    i += 1;
                }
                if i >= len {
                    return Err("Unterminated string literal".to_string());
                }
                let str_val: String = chars[start..i].iter().collect();
                tokens.push(Token::Str(str_val));
                i += 1; // consume closing quote
            }
            _ => {
                if c.is_digit(10) || (c == '.' && i + 1 < len && chars[i + 1].is_digit(10)) {
                    let start = i;
                    while i < len && (chars[i].is_digit(10) || chars[i] == '.') {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    let num: f64 = num_str
                        .parse()
                        .map_err(|e| format!("Invalid numeric literal '{}': {}", num_str, e))?;
                    tokens.push(Token::Number(num));
                } else if c.is_alphabetic() || c == '_' {
                    let start = i;
                    while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let word: String = chars[start..i].iter().collect();
                    match word.to_uppercase().as_str() {
                        "SELECT" => tokens.push(Token::Select),
                        "WHERE" => tokens.push(Token::Where),
                        "GROUP" => tokens.push(Token::Group),
                        "BY" => tokens.push(Token::By),
                        "SUM" => tokens.push(Token::Sum),
                        "COUNT" => tokens.push(Token::Count),
                        "AVG" => tokens.push(Token::Avg),
                        _ => tokens.push(Token::Ident(word)),
                    }
                } else {
                    return Err(format!("Unexpected character in query: '{}'", c));
                }
            }
        }
    }

    Ok(tokens)
}

fn parse_select_list(tokens: &[Token], pos: &mut usize) -> Result<Vec<SelectExpr>, String> {
    let mut exprs = Vec::new();

    loop {
        if *pos >= tokens.len() {
            return Err("Unexpected end of query in SELECT list".to_string());
        }

        let expr = parse_select_expr(tokens, pos)?;
        exprs.push(expr);

        if *pos < tokens.len() && tokens[*pos] == Token::Comma {
            *pos += 1; // consume comma
        } else {
            break;
        }
    }

    if exprs.is_empty() {
        return Err("SELECT list cannot be empty".to_string());
    }

    Ok(exprs)
}

fn parse_select_expr(tokens: &[Token], pos: &mut usize) -> Result<SelectExpr, String> {
    if *pos >= tokens.len() {
        return Err("Expected expression in SELECT list".to_string());
    }

    match &tokens[*pos] {
        Token::Star => {
            *pos += 1;
            Ok(SelectExpr::Star)
        }
        Token::Sum => {
            *pos += 1; // consume SUM
            expect_token(tokens, pos, Token::LParen, "(")?;
            let col = parse_column(tokens, pos)?;
            expect_token(tokens, pos, Token::RParen, ")")?;
            Ok(SelectExpr::Agg(AggregateFunc::Sum(col)))
        }
        Token::Count => {
            *pos += 1; // consume COUNT
            expect_token(tokens, pos, Token::LParen, "(")?;
            if *pos < tokens.len() && tokens[*pos] == Token::Star {
                *pos += 1;
            } else {
                return Err("COUNT currently only supports COUNT(*)".to_string());
            }
            expect_token(tokens, pos, Token::RParen, ")")?;
            Ok(SelectExpr::Agg(AggregateFunc::CountStar))
        }
        Token::Avg => {
            *pos += 1; // consume AVG
            expect_token(tokens, pos, Token::LParen, "(")?;
            let col = parse_column(tokens, pos)?;
            expect_token(tokens, pos, Token::RParen, ")")?;
            Ok(SelectExpr::Agg(AggregateFunc::Avg(col)))
        }
        Token::Ident(name) => {
            let col = Column::from_str(name)?;
            *pos += 1;
            Ok(SelectExpr::Col(col))
        }
        tok => Err(format!("Unexpected token in SELECT clause: '{:?}'", tok)),
    }
}

fn parse_where_clause(tokens: &[Token], pos: &mut usize) -> Result<FilterCondition, String> {
    if *pos >= tokens.len() {
        return Err("Expected column in WHERE clause".to_string());
    }

    let col = parse_column(tokens, pos)?;

    if *pos >= tokens.len() {
        return Err("Expected operator in WHERE clause".to_string());
    }

    let op = match &tokens[*pos] {
        Token::Operator(op) => *op,
        tok => return Err(format!("Expected comparison operator, found '{:?}'", tok)),
    };
    *pos += 1;

    if *pos >= tokens.len() {
        return Err("Expected literal value in WHERE clause".to_string());
    }

    let lit = match &tokens[*pos] {
        Token::Number(num) => Value::Number(*num),
        Token::Str(s) => {
            if let Ok(dept) = Department::from_str(s) {
                Value::Dept(dept)
            } else {
                Value::Str(s.clone())
            }
        }
        Token::Ident(s) => {
            if let Ok(dept) = Department::from_str(s) {
                Value::Dept(dept)
            } else {
                Value::Str(s.clone())
            }
        }
        tok => {
            return Err(format!(
                "Expected literal value in WHERE clause, found '{:?}'",
                tok
            ))
        }
    };
    *pos += 1;

    Ok(FilterCondition {
        column: col,
        operator: op,
        literal: lit,
    })
}

fn parse_column(tokens: &[Token], pos: &mut usize) -> Result<Column, String> {
    if *pos >= tokens.len() {
        return Err("Expected column name".to_string());
    }

    if let Token::Ident(name) = &tokens[*pos] {
        let col = Column::from_str(name)?;
        *pos += 1;
        Ok(col)
    } else {
        Err(format!(
            "Expected column identifier, found '{:?}'",
            tokens[*pos]
        ))
    }
}

fn expect_token(
    tokens: &[Token],
    pos: &mut usize,
    expected: Token,
    name: &str,
) -> Result<(), String> {
    if *pos < tokens.len() && tokens[*pos] == expected {
        *pos += 1;
        Ok(())
    } else {
        Err(format!("Expected '{}'", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_queries() {
        let q1 =
            parse_query("SELECT department, SUM(salary) WHERE salary > 100000 GROUP BY department")
                .unwrap();
        assert_eq!(q1.select.len(), 2);
        assert_eq!(q1.group_by, Some(Column::Department));
        assert!(q1.filter.is_some());

        let q2 = parse_query("SELECT department, COUNT(*) GROUP BY department").unwrap();
        assert_eq!(q2.select[1], SelectExpr::Agg(AggregateFunc::CountStar));

        let q3 = parse_query("SELECT department, AVG(salary) WHERE age >= 30 GROUP BY department")
            .unwrap();
        assert_eq!(q3.filter.unwrap().operator, Operator::Gte);
    }

    #[test]
    fn test_parse_invalid_syntax() {
        assert!(parse_query("INVALID QUERY").is_err());
        assert!(parse_query("SELECT").is_err());
        assert!(parse_query("SELECT WHERE salary > 100").is_err());
        assert!(parse_query("SELECT department, SUM(salary) GROUP").is_err());
    }
}
