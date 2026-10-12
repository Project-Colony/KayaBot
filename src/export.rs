//! The rename summary as a CSV or JSON file.

use std::path::Path;
use std::{fs, io};

use serde::Serialize;

/// One renamed, skipped or failed file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExportRow {
    /// `ok` or `error`: stable values a script can test, whatever the locale.
    pub status: &'static str,
    pub message: String,
    pub final_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    /// The name the save dialog proposes.
    pub fn default_file_name(self) -> &'static str {
        match self {
            Self::Csv => "rename-summary.csv",
            Self::Json => "rename-summary.json",
        }
    }

    pub fn render(self, rows: &[ExportRow]) -> String {
        match self {
            Self::Csv => to_csv(rows),
            Self::Json => to_json(rows),
        }
    }
}

pub fn write(path: &Path, format: ExportFormat, rows: &[ExportRow]) -> io::Result<()> {
    fs::write(path, format.render(rows))
}

fn to_csv(rows: &[ExportRow]) -> String {
    let mut output = String::from("status,message,final_path\n");
    for row in rows {
        output.push_str(&format!(
            "{},{},{}\n",
            csv_escape(row.status),
            csv_escape(&row.message),
            csv_escape(&row.final_path)
        ));
    }
    output
}

fn to_json(rows: &[ExportRow]) -> String {
    // A Vec of plain strings always serializes.
    serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
}

fn csv_escape(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<ExportRow> {
        vec![
            ExportRow {
                status: "ok",
                message: "Renamed to Final Name.mkv".to_string(),
                final_path: "/media/Final Name.mkv".to_string(),
            },
            ExportRow {
                status: "error",
                message: "Failed: \"disk\", full".to_string(),
                final_path: "/media/missing.mkv".to_string(),
            },
        ]
    }

    #[test]
    fn csv_has_a_header_and_quotes_what_needs_it() {
        let csv = ExportFormat::Csv.render(&rows());
        assert_eq!(
            csv,
            "status,message,final_path\n\
             ok,Renamed to Final Name.mkv,/media/Final Name.mkv\n\
             error,\"Failed: \"\"disk\"\", full\",/media/missing.mkv\n"
        );
    }

    #[test]
    fn json_is_an_array_of_rows() {
        let json = ExportFormat::Json.render(&rows());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).expect("valid json");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["status"], "ok");
        assert_eq!(parsed[1]["final_path"], "/media/missing.mkv");
    }
}
