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
    fn test_output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Text);
    }

    #[test]
    fn test_print_json() {
        // Just verify it doesn't panic
        #[derive(Serialize)]
        struct Test { value: i32 }
        print_json(&Test { value: 42 });
    }

    #[test]
    fn test_print_pairs() {
        // Just verify it doesn't panic
        print_pairs(&[("Key", "Value".to_string())]);
    }
}
