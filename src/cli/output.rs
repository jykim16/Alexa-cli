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
