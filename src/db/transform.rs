use tracing::debug;

/// Transforms SQL dump to be compatible with Supabase target project
pub struct SqlTransformer;

impl SqlTransformer {
    /// Apply all Supabase-specific transformations to SQL dump
    pub fn transform(sql: &str) -> String {
        let mut result = sql.to_string();

        // Comment out auth schema operations (managed by Supabase)
        result = Self::comment_line(&result, "DROP SCHEMA IF EXISTS \"auth\";");
        result = Self::comment_line(&result, "CREATE SCHEMA \"auth\";");

        // Comment out storage schema operations (managed by Supabase)
        result = Self::comment_line(&result, "DROP SCHEMA IF EXISTS \"storage\";");
        result = Self::comment_line(&result, "CREATE SCHEMA \"storage\";");

        // Comment out supabase_admin default privileges
        result = Self::comment_lines_starting_with(
            &result,
            "ALTER DEFAULT PRIVILEGES FOR ROLE \"supabase_admin\"",
        );

        debug!("Applied SQL transformations for Supabase compatibility");
        result
    }

    /// Comment out a specific line
    fn comment_line(sql: &str, target: &str) -> String {
        sql.lines()
            .map(|line| {
                if line.trim() == target {
                    format!("-- {}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Comment out all lines starting with a pattern
    fn comment_lines_starting_with(sql: &str, pattern: &str) -> String {
        sql.lines()
            .map(|line| {
                if line.trim().starts_with(pattern) {
                    format!("-- {}", line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_auth_schema() {
        let input = r#"
DROP SCHEMA IF EXISTS "auth";
CREATE SCHEMA "auth";
DROP SCHEMA IF EXISTS "public";
"#;
        let result = SqlTransformer::transform(input);
        assert!(result.contains("-- DROP SCHEMA IF EXISTS \"auth\";"));
        assert!(result.contains("-- CREATE SCHEMA \"auth\";"));
        assert!(result.contains("DROP SCHEMA IF EXISTS \"public\";"));
    }

    #[test]
    fn test_comment_storage_schema() {
        let input = r#"
DROP SCHEMA IF EXISTS "storage";
CREATE SCHEMA "storage";
"#;
        let result = SqlTransformer::transform(input);
        assert!(result.contains("-- DROP SCHEMA IF EXISTS \"storage\";"));
        assert!(result.contains("-- CREATE SCHEMA \"storage\";"));
    }

    #[test]
    fn test_comment_supabase_admin() {
        let input = r#"
ALTER DEFAULT PRIVILEGES FOR ROLE "supabase_admin" IN SCHEMA "public" GRANT ALL ON TABLES TO "postgres";
ALTER DEFAULT PRIVILEGES FOR ROLE "supabase_admin" IN SCHEMA "public" GRANT ALL ON SEQUENCES TO "postgres";
"#;
        let result = SqlTransformer::transform(input);
        assert!(result.contains("-- ALTER DEFAULT PRIVILEGES FOR ROLE \"supabase_admin\""));
    }
}
