use std::borrow::Cow;

const READ_ONLY_START_KEYWORDS: &[&str] =
    &["SELECT", "SHOW", "DESC", "DESCRIBE", "EXPLAIN", "WITH"];

const WRITE_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "UPSERT", "REPLACE", "MERGE", "CREATE", "ALTER", "DROP",
    "TRUNCATE", "GRANT", "REVOKE", "COMMIT", "ROLLBACK", "BEGIN", "START", "VACUUM", "ANALYZE",
    "ATTACH", "DETACH", "PRAGMA", "EXEC", "EXECUTE", "CALL", "DO", "SET", "USE", "LOCK", "UNLOCK",
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
mod test{
    use crate::read_only::is_read_only_sql;
    // ===================== Basic SELECT =====================
    
    #[test]
    fn select_all_from_table() {
        assert!(is_read_only_sql("SELECT * FROM users"));
    }
    
    #[test]
    fn select_with_where_clause() {
        assert!(is_read_only_sql("SELECT id, name FROM users WHERE age > 18"));
    }
    
    #[test]
    fn select_with_multiple_conditions() {
        assert!(is_read_only_sql("SELECT * FROM orders WHERE status = 'active' AND created_at > '2024-01-01'"));
    }
    
    #[test]
    fn select_with_join() {
        assert!(is_read_only_sql("SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id"));
    }
    
    #[test]
    fn select_with_group_by_and_having() {
        assert!(is_read_only_sql("SELECT department, COUNT(*) FROM employees GROUP BY department HAVING COUNT(*) > 5"));
    }
    
    #[test]
    fn select_with_order_by_and_limit() {
        assert!(is_read_only_sql("SELECT name, salary FROM employees ORDER BY salary DESC LIMIT 10"));
    }
    
    #[test]
    fn select_with_subquery() {
        assert!(is_read_only_sql("SELECT name FROM users WHERE id IN (SELECT user_id FROM orders)"));
    }
    
    #[test]
    fn select_distinct() {
        assert!(is_read_only_sql("SELECT DISTINCT city FROM customers"));
    }
    
    #[test]
    fn select_with_alias() {
        assert!(is_read_only_sql("SELECT e.name AS employee_name FROM employees e"));
    }
    
    #[test]
    fn select_with_case_when() {
        assert!(is_read_only_sql("SELECT name, CASE WHEN age >= 18 THEN 'adult' ELSE 'minor' END AS status FROM users"));
    }
    
    #[test]
    fn select_with_function_call() {
        assert!(is_read_only_sql("SELECT COUNT(*), AVG(price), MAX(quantity) FROM products"));
    }
    
    #[test]
    fn select_with_numeric_ops() {
        assert!(is_read_only_sql("SELECT price * quantity AS total FROM order_items"));
    }
    
    #[test]
    fn select_all_from_specific_schema() {
        assert!(is_read_only_sql("SELECT * FROM public.users"));
    }
    
    #[test]
    fn select_with_window_function() {
        assert!(is_read_only_sql("SELECT name, salary, RANK() OVER (ORDER BY salary DESC) AS rank FROM employees"));
    }
    
    #[test]
    fn select_with_union() {
        assert!(is_read_only_sql("SELECT name FROM users UNION SELECT name FROM admins"));
    }
    
    #[test]
    fn select_with_intersect() {
        assert!(is_read_only_sql("SELECT name FROM users INTERSECT SELECT name FROM admins"));
    }
    
    #[test]
    fn select_with_except() {
        assert!(is_read_only_sql("SELECT name FROM users EXCEPT SELECT name FROM banned_users"));
    }
    
    #[test]
    fn select_with_escaped_quote() {
        assert!(is_read_only_sql("SELECT 'it\\'s a test' AS val"));
    }
    
    #[test]
    fn select_with_unicode_string() {
        assert!(is_read_only_sql("SELECT '中文' AS greeting, '日本語' AS lang"));
    }
    
    #[test]
    fn select_with_special_chars_in_identifier() {
        assert!(is_read_only_sql("SELECT * FROM \"my-table\" WHERE \"col§1\" = 42"));
    }
    
    #[test]
    fn select_across_multiple_lines() {
        assert!(is_read_only_sql("SELECT\n  id,\n  name\nFROM\n  users\nWHERE\n  age > 18"));
    }
    
    #[test]
    fn select_with_string_semicolon() {
        assert!(is_read_only_sql("SELECT ';' AS semi"));
    }
    
    #[test]
    fn select_with_multiple_semicolons_only_in_string() {
        assert!(is_read_only_sql("SELECT ';;;' AS semis"));
    }
    
    #[test]
    fn select_with_trailing_semicolon() {
        assert!(is_read_only_sql("SELECT * FROM users;"));
    }
    
    #[test]
    fn select_semicolon_with_whitespace_before_eof() {
        assert!(is_read_only_sql("SELECT * FROM users;  "));
    }
    
    #[test]
    fn lowercase_select() {
        assert!(is_read_only_sql("select * from users"));
    }
    
    #[test]
    fn mixed_case_select() {
        assert!(is_read_only_sql("SeLeCt * FrOm users"));
    }
    
    #[test]
    fn select_with_leading_whitespace() {
        assert!(is_read_only_sql("  SELECT * FROM users"));
    }
    
    #[test]
    fn select_with_trailing_newline() {
        assert!(is_read_only_sql("SELECT * FROM users\n"));
    }
    
    // ===================== SHOW / DESC / EXPLAIN =====================
    
    #[test]
    fn show_tables() {
        assert!(is_read_only_sql("SHOW TABLES"));
    }
    
    #[test]
    fn show_databases() {
        assert!(is_read_only_sql("SHOW DATABASES"));
    }
    
    
    #[test]
    fn show_variables() {
        assert!(is_read_only_sql("SHOW VARIABLES LIKE 'character_set%'"));
    }
    
    #[test]
    fn show_with_like() {
        assert!(is_read_only_sql("SHOW TABLES LIKE '%user%'"));
    }
    
    #[test]
    fn describe_short() {
        assert!(is_read_only_sql("DESC users"));
    }
    
    #[test]
    fn describe_full() {
        assert!(is_read_only_sql("DESCRIBE users"));
    }
    
    #[test]
    fn explain_select() {
        assert!(is_read_only_sql("EXPLAIN SELECT * FROM users"));
    }
    
    // ===================== WITH (CTE) read-only =====================
    
    #[test]
    fn with_cte_read_only() {
        assert!(is_read_only_sql("WITH active_users AS (SELECT * FROM users WHERE status = 'active') SELECT * FROM active_users"));
    }
    
    #[test]
    fn with_recursive_cte() {
        assert!(is_read_only_sql("WITH RECURSIVE numbers AS (SELECT 1 AS n UNION ALL SELECT n + 1 FROM numbers WHERE n < 10) SELECT * FROM numbers"));
    }
    
    // ===================== Reject INSERT =====================
    
    #[test]
    fn reject_insert_values() {
        assert!(!is_read_only_sql("INSERT INTO users (name, email) VALUES ('John', 'john@example.com')"));
    }
    
    #[test]
    fn reject_insert_select() {
        assert!(!is_read_only_sql("INSERT INTO archive SELECT * FROM old_orders WHERE created_at < '2020-01-01'"));
    }
    
    #[test]
    fn reject_insert_multi_row() {
        assert!(!is_read_only_sql("INSERT INTO products (name, price) VALUES ('A', 10), ('B', 20), ('C', 30)"));
    }
    
    #[test]
    fn reject_insert_on_conflict() {
        assert!(!is_read_only_sql("INSERT INTO users (id, name) VALUES (1, 'John') ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name"));
    }
    
    #[test]
    fn reject_upsert() {
        assert!(!is_read_only_sql("UPSERT INTO users (id, name) VALUES (1, 'John')"));
    }
    
    #[test]
    fn reject_replace() {
        assert!(!is_read_only_sql("REPLACE INTO users (id, name) VALUES (1, 'John')"));
    }
    
    #[test]
    fn lowercase_insert() {
        assert!(!is_read_only_sql("insert into users values (1)"));
    }
    
    #[test]
    fn reject_insert_across_lines() {
        assert!(!is_read_only_sql("INSERT INTO\n  users (id, name)\nVALUES\n  (1, 'John')"));
    }
    
    // ===================== Reject UPDATE =====================
    
    #[test]
    fn reject_update_basic() {
        assert!(!is_read_only_sql("UPDATE users SET name = 'John' WHERE id = 1"));
    }
    
    #[test]
    fn reject_update_all_rows() {
        assert!(!is_read_only_sql("UPDATE products SET price = price * 1.1"));
    }
    
    #[test]
    fn reject_update_with_subquery() {
        assert!(!is_read_only_sql("UPDATE users u SET orders_count = (SELECT COUNT(*) FROM orders WHERE user_id = u.id)"));
    }
    
    #[test]
    fn reject_update_with_returning() {
        assert!(!is_read_only_sql("UPDATE employees SET salary = salary * 1.1 WHERE department = 'Engineering' RETURNING *"));
    }
    
    // ===================== Reject DELETE =====================
    
    #[test]
    fn reject_delete_basic() {
        assert!(!is_read_only_sql("DELETE FROM users WHERE id = 1"));
    }
    
    #[test]
    fn reject_delete_all() {
        assert!(!is_read_only_sql("DELETE FROM logs"));
    }
    
    #[test]
    fn reject_delete_with_returning() {
        assert!(!is_read_only_sql("DELETE FROM orders WHERE status = 'cancelled' RETURNING id"));
    }
    
    #[test]
    fn mixed_case_delete() {
        assert!(!is_read_only_sql("DeLeTe FrOm users"));
    }
    
    // ===================== Reject DDL =====================
    
    #[test]
    fn reject_create_table() {
        assert!(!is_read_only_sql("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)"));
    }
    
    #[test]
    fn reject_create_index() {
        assert!(!is_read_only_sql("CREATE INDEX idx_users_name ON users(name)"));
    }
    
    #[test]
    fn reject_create_view() {
        assert!(!is_read_only_sql("CREATE VIEW active_users AS SELECT * FROM users WHERE status = 'active'"));
    }
    
    #[test]
    fn reject_create_database() {
        assert!(!is_read_only_sql("CREATE DATABASE test_db"));
    }
    
    #[test]
    fn reject_alter_table_add_column() {
        assert!(!is_read_only_sql("ALTER TABLE users ADD COLUMN age INT"));
    }
    
    #[test]
    fn reject_alter_table_drop_column() {
        assert!(!is_read_only_sql("ALTER TABLE users DROP COLUMN age"));
    }
    
    #[test]
    fn reject_alter_table_modify() {
        assert!(!is_read_only_sql("ALTER TABLE users MODIFY COLUMN name VARCHAR(200)"));
    }
    
    #[test]
    fn reject_drop_table() {
        assert!(!is_read_only_sql("DROP TABLE users"));
    }
    
    #[test]
    fn reject_drop_index() {
        assert!(!is_read_only_sql("DROP INDEX idx_users_name"));
    }
    
    #[test]
    fn reject_drop_view() {
        assert!(!is_read_only_sql("DROP VIEW active_users"));
    }
    
    #[test]
    fn reject_drop_database() {
        assert!(!is_read_only_sql("DROP DATABASE test_db"));
    }
    
    #[test]
    fn reject_truncate() {
        assert!(!is_read_only_sql("TRUNCATE TABLE logs"));
    }
    
    // ===================== Reject MERGE / CALL / EXEC =====================
    
    #[test]
    fn reject_merge() {
        assert!(!is_read_only_sql("MERGE INTO target t USING source s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.name = s.name"));
    }
    
    #[test]
    fn reject_call_procedure() {
        assert!(!is_read_only_sql("CALL calculate_salary()"));
    }
    
    #[test]
    fn reject_exec() {
        assert!(!is_read_only_sql("EXEC sp_helpdb"));
    }
    
    #[test]
    fn reject_execute() {
        assert!(!is_read_only_sql("EXECUTE sp_helpdb"));
    }
    
    // ===================== Reject Transaction control =====================
    
    #[test]
    fn reject_begin_transaction() {
        assert!(!is_read_only_sql("BEGIN TRANSACTION"));
    }
    
    #[test]
    fn reject_begin_work() {
        assert!(!is_read_only_sql("BEGIN WORK"));
    }
    
    #[test]
    fn reject_start_transaction() {
        assert!(!is_read_only_sql("START TRANSACTION"));
    }
    
    #[test]
    fn reject_commit() {
        assert!(!is_read_only_sql("COMMIT"));
    }
    
    #[test]
    fn reject_rollback() {
        assert!(!is_read_only_sql("ROLLBACK"));
    }
    
    // ===================== Reject other operations =====================
    
    #[test]
    fn reject_grant() {
        assert!(!is_read_only_sql("GRANT SELECT ON users TO readonly_user"));
    }
    
    #[test]
    fn reject_revoke() {
        assert!(!is_read_only_sql("REVOKE INSERT ON users FROM readonly_user"));
    }
    
    #[test]
    fn reject_set_variable() {
        assert!(!is_read_only_sql("SET @variable = 1"));
    }
    
    #[test]
    fn reject_use_database() {
        assert!(!is_read_only_sql("USE test_db"));
    }
    
    #[test]
    fn reject_lock_table() {
        assert!(!is_read_only_sql("LOCK TABLE users IN EXCLUSIVE MODE"));
    }
    
    #[test]
    fn reject_lock_tables() {
        assert!(!is_read_only_sql("LOCK TABLES users WRITE"));
    }
    
    #[test]
    fn reject_unlock_tables() {
        assert!(!is_read_only_sql("UNLOCK TABLES"));
    }
    
    #[test]
    fn reject_vacuum() {
        assert!(!is_read_only_sql("VACUUM"));
    }
    
    #[test]
    fn reject_analyze() {
        assert!(!is_read_only_sql("ANALYZE users"));
    }
    
    #[test]
    fn reject_pragma_read() {
        assert!(!is_read_only_sql("PRAGMA table_info('users')"));
    }
    
    #[test]
    fn reject_pragma_write() {
        assert!(!is_read_only_sql("PRAGMA journal_mode=WAL"));
    }
    
    #[test]
    fn reject_do_block() {
        assert!(!is_read_only_sql("DO $$ BEGIN PERFORM 1; END $$"));
    }
    
    // ===================== Reject multi-statement =====================
    
    #[test]
    fn reject_two_selects_with_semicolon() {
        assert!(!is_read_only_sql("SELECT * FROM users; SELECT * FROM orders"));
    }
    
    #[test]
    fn reject_select_then_insert() {
        assert!(!is_read_only_sql("SELECT * FROM users; INSERT INTO logs VALUES ('queried')"));
    }
    
    #[test]
    fn reject_three_statements() {
        assert!(!is_read_only_sql("SELECT 1; SELECT 2; SELECT 3"));
    }
    
    #[test]
    fn reject_insert_then_select() {
        assert!(!is_read_only_sql("INSERT INTO users VALUES (1); SELECT * FROM users"));
    }
    
    // ===================== Edge cases: write keyword in literals =====================
    
    #[test]
    fn select_with_insert_in_string() {
        assert!(is_read_only_sql("SELECT * FROM users WHERE name = 'INSERT'"));
    }
    
    #[test]
    fn select_with_update_in_string() {
        assert!(is_read_only_sql("SELECT * FROM users WHERE name = 'UPDATE'"));
    }
    
    #[test]
    fn select_with_delete_in_string() {
        assert!(is_read_only_sql("SELECT * FROM users WHERE name = 'DELETE'"));
    }
    
    #[test]
    fn select_with_drop_in_string() {
        assert!(is_read_only_sql("SELECT 'DROP TABLE' AS msg"));
    }
    
    #[test]
    fn select_with_write_keyword_in_double_quoted_id() {
        assert!(is_read_only_sql("SELECT \"INSERT\" AS col"));
    }
    
    #[test]
    fn select_with_write_keyword_in_backtick_id() {
        assert!(is_read_only_sql("SELECT `DELETE` FROM users"));
    }
    
    #[test]
    fn select_with_write_keyword_in_column_name() {
        assert!(is_read_only_sql("SELECT t.delete_flag FROM users t"));
    }
    
    // ===================== Edge cases: comments =====================
    
    #[test]
    fn select_with_line_comment() {
        assert!(is_read_only_sql("SELECT * FROM users -- this is a comment"));
    }
    
    #[test]
    fn select_with_block_comment() {
        assert!(is_read_only_sql("SELECT * /* comment */ FROM users"));
    }
    
    #[test]
    fn reject_delete_after_line_comment() {
        assert!(!is_read_only_sql("-- comment\nDELETE FROM users"));
    }
    
    #[test]
    fn reject_insert_masked_by_block_comment() {
        assert!(!is_read_only_sql("/* hi */ INSERT INTO users VALUES (1)"));
    }
    
    #[test]
    fn reject_block_comment_then_update() {
        assert!(!is_read_only_sql("/* block */ UPDATE users SET name = 'test'"));
    }
    
    // ===================== Edge cases: write keyword inside CTE =====================
    
    #[test]
    fn reject_cte_with_delete() {
        assert!(!is_read_only_sql(
            "WITH deleted AS (DELETE FROM users RETURNING *) SELECT * FROM deleted"
        ));
    }
    
    #[test]
    fn reject_cte_with_update() {
        assert!(!is_read_only_sql(
            "WITH updated AS (UPDATE products SET price = 0 RETURNING *) SELECT * FROM updated"
        ));
    }
    
    #[test]
    fn reject_cte_with_insert() {
        assert!(!is_read_only_sql(
            "WITH inserted AS (INSERT INTO logs VALUES (1) RETURNING *) SELECT * FROM inserted"
        ));
    }
    
    // ===================== Edge cases: empty / invalid =====================
    
    #[test]
    fn empty_string() {
        assert!(!is_read_only_sql(""));
    }
    
    #[test]
    fn whitespace_only() {
        assert!(!is_read_only_sql("   \t\n  "));
    }
    
    #[test]
    fn expression_not_starting_with_read_keyword() {
        assert!(!is_read_only_sql("1 + 1"));
    }
    
    #[test]
    fn simple_number() {
        assert!(!is_read_only_sql("42"));
    }
    
    #[test]
    fn function_call_no_select() {
        assert!(!is_read_only_sql("NOW()"));
    }
    
    // ===================== SELECT ... FOR UPDATE =====================
    
    #[test]
    fn select_for_update_is_rejected() {
        // UPDATE appears in write keywords, so FOR UPDATE triggers rejection
        assert!(!is_read_only_sql("SELECT * FROM users FOR UPDATE"));
    }
    
    // ===================== Verify all WRITE_KEYWORDS =====================
    
    #[test]
    fn all_write_keywords_rejected_as_first_word() {
        let write_keywords = [
            "INSERT", "UPDATE", "DELETE", "UPSERT", "REPLACE", "MERGE",
            "CREATE", "ALTER", "DROP", "TRUNCATE", "GRANT", "REVOKE",
            "COMMIT", "ROLLBACK", "BEGIN", "START", "VACUUM", "ANALYZE",
            "ATTACH", "DETACH", "PRAGMA", "EXEC", "EXECUTE", "CALL",
            "DO", "SET", "USE", "LOCK", "UNLOCK",
        ];
        for kw in &write_keywords {
            let sql = format!("{} TABLE test", kw);
            assert!(!is_read_only_sql(&sql), "expected {} to be rejected", kw);
        }
    }
    
    // ===================== Verify all READ_ONLY_START_KEYWORDS =====================
    
    #[test]
    fn all_read_only_keywords_accepted() {
        let read_keywords = ["SELECT", "SHOW", "DESC", "DESCRIBE", "EXPLAIN", "WITH"];
        for kw in &read_keywords {
            let sql = if *kw == "WITH" {
                "WITH cte AS (SELECT 1) SELECT * FROM cte".to_string()
            } else {
                format!("{} 1", kw)
            };
            assert!(is_read_only_sql(&sql), "expected {} to be accepted", kw);
        }
    }
}