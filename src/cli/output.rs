use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Table,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

/// Print a value as JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    );
}

/// Print a success message.
pub fn print_success(msg: &str) {
    println!("{}", msg);
}

/// Print a list of (label, value) pairs as a simple table.
pub fn print_pairs(pairs: &[(&str, String)]) {
    let max_len = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (k, v) in pairs {
        println!("{:<width$}  {}", k, v, width = max_len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default_is_text() {
        let fmt = OutputFormat::default();
        assert_eq!(fmt, OutputFormat::Text);
    }

    #[test]
    fn test_print_json_serializes_struct() {
        #[derive(serde::Serialize)]
        struct Foo {
            name: &'static str,
            count: u32,
        }
        // Just verify it doesn't panic and produces valid JSON
        let foo = Foo { name: "test", count: 42 };
        // Capture output by calling the serializer directly
        let json = serde_json::to_string_pretty(&foo).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_print_pairs_aligns_on_longest_key() {
        // Smoke test: just ensure it does not panic with mixed-length keys
        let pairs = &[
            ("Short", "val1".to_string()),
            ("A very long key", "val2".to_string()),
        ];
        print_pairs(pairs); // must not panic
    }

    #[test]
    fn test_print_pairs_empty_slice() {
        print_pairs(&[]); // must not panic
    }

    #[test]
    fn test_output_format_variants_distinct() {
        assert_ne!(OutputFormat::Text, OutputFormat::Json);
        assert_ne!(OutputFormat::Json, OutputFormat::Table);
        assert_ne!(OutputFormat::Text, OutputFormat::Table);
    }
}
