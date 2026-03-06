use std::borrow::Cow;

const READ_ONLY_START_KEYWORDS: &[&str] = &[
    "SELECT",
    "SHOW",
    "DESC",
    "DESCRIBE",
    "EXPLAIN",
    "WITH",
];

const WRITE_KEYWORDS: &[&str] = &[
    "INSERT",
    "UPDATE",
    "DELETE",
    "UPSERT",
    "REPLACE",
    "MERGE",
    "CREATE",
    "ALTER",
    "DROP",
    "TRUNCATE",
    "GRANT",
    "REVOKE",
    "COMMIT",
    "ROLLBACK",
    "BEGIN",
    "START",
    "VACUUM",
    "ANALYZE",
    "ATTACH",
    "DETACH",
    "PRAGMA",
    "EXEC",
    "EXECUTE",
    "CALL",
    "DO",
    "SET",
    "USE",
    "LOCK",
    "UNLOCK",
];

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_escaped(sql: &[char], pos: usize) -> bool {
    if pos == 0 {
        return false;
    }
    let mut backslashes = 0usize;
    let mut i = pos;
    while i > 0 {
        i -= 1;
        if sql[i] == '\\' {
            backslashes += 1;
        } else {
            break;
        }
    }
    backslashes % 2 == 1
}

fn collect_uppercase_words_and_semicolons(sql: &str) -> (Vec<String>, usize) {
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0usize;
    let mut words = Vec::new();
    let mut semicolons = 0usize;

    while i < chars.len() {
        let c = chars[i];
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '-' && next == Some('-') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if c == '\'' || c == '"' || c == '`' {
            let quote = c;
            i += 1;
            while i < chars.len() {
                if chars[i] == quote && !is_escaped(&chars, i) {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if c == ';' {
            semicolons += 1;
            i += 1;
            continue;
        }

        if is_ident_char(c) {
            let start = i;
            i += 1;
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            let token: Cow<'_, str> = chars[start..i].iter().collect::<String>().into();
            words.push(token.to_ascii_uppercase());
            continue;
        }

        i += 1;
    }

    (words, semicolons)
}

pub fn is_read_only_sql(sql: &str) -> bool {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return false;
    }

    let (words, semicolons) = collect_uppercase_words_and_semicolons(trimmed);
    if words.is_empty() {
        return false;
    }

    if semicolons > 1 {
        return false;
    }

    if semicolons == 1 && !trimmed.ends_with(';') {
        return false;
    }

    let first = words.first().map(|s| s.as_str()).unwrap_or_default();
    if !READ_ONLY_START_KEYWORDS.contains(&first) {
        return false;
    }

    for word in &words {
        if WRITE_KEYWORDS.contains(&word.as_str()) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::is_read_only_sql;

    #[test]
    fn allows_plain_select() {
        assert!(is_read_only_sql("SELECT * FROM users"));
    }

    #[test]
    fn allows_select_with_string_semicolon() {
        assert!(is_read_only_sql("SELECT ';' AS semi"));
    }

    #[test]
    fn allows_explain_select() {
        assert!(is_read_only_sql("EXPLAIN SELECT * FROM users"));
    }

    #[test]
    fn rejects_delete() {
        assert!(!is_read_only_sql("DELETE FROM users WHERE id = 1"));
    }

    #[test]
    fn rejects_comment_prefixed_delete() {
        assert!(!is_read_only_sql("-- note\nDELETE FROM users"));
    }

    #[test]
    fn rejects_multi_statement() {
        assert!(!is_read_only_sql("SELECT * FROM users; DELETE FROM users"));
    }

    #[test]
    fn rejects_write_cte() {
        assert!(!is_read_only_sql(
            "WITH moved AS (DELETE FROM users RETURNING *) SELECT * FROM moved"
        ));
    }
}
