/*!
 * JSONL streaming for Claude Code logs
 *
 * Simple line-by-line JSONL parser with O(1) memory usage.
 * Unlike floatctl-core's JsonArrayStream, Claude logs are already NDJSON.
 */

use crate::LogEntry;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Iterator over log entries in a JSONL file
pub struct LogStream {
    reader: BufReader<File>,
    line_number: usize,
}

impl LogStream {
    /// Create a new log stream from a file path
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open log file: {}", path.display()))?;

        Ok(Self {
            reader: BufReader::new(file),
            line_number: 0,
        })
    }

    /// Read the next log entry
    pub fn next_entry(&mut self) -> Result<Option<LogEntry>> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;

            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            self.line_number += 1;

            // Skip empty lines
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse JSON
            let entry: LogEntry = serde_json::from_str(trimmed).with_context(|| {
                // Also print to stderr for debugging
                eprintln!("Parse error at line {}: {}", self.line_number, trimmed.chars().take(200).collect::<String>());
                format!(
                    "Failed to parse log entry at line {}: {}",
                    self.line_number,
                    trimmed.chars().take(100).collect::<String>()
                )
            })?;

            return Ok(Some(entry));
        }
    }

    /// Collect all entries into a Vec (loads entire file into memory)
    /// Use sparingly - prefer iterating with next_entry() for large files
    pub fn collect_all(mut self) -> Result<Vec<LogEntry>> {
        let mut entries = Vec::new();
        while let Some(entry) = self.next_entry()? {
            entries.push(entry);
        }
        Ok(entries)
    }
}

/// Helper: Read all log entries from a file
/// Convenience wrapper around LogStream::collect_all()
pub fn read_log_file(path: &Path) -> Result<Vec<LogEntry>> {
    LogStream::new(path)?.collect_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_log() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;

        // Write sample JSONL entries
        writeln!(
            file,
            r#"{{"type":"queue-operation","operation":"enqueue","timestamp":"2025-11-09T01:13:40.906Z","content":"test message","sessionId":"abc123"}}"#
        )?;
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2025-11-09T01:13:40.943Z","sessionId":"abc123"}}"#
        )?;

        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_log_stream_basic() -> Result<()> {
        let file = create_test_log()?;
        let mut stream = LogStream::new(file.path())?;

        // Read first entry
        let entry1 = stream.next_entry()?.expect("Should have first entry");
        assert_eq!(entry1.entry_type, "queue-operation");
        assert_eq!(entry1.session_id.as_deref(), Some("abc123"));

        // Read second entry
        let entry2 = stream.next_entry()?.expect("Should have second entry");
        assert_eq!(entry2.entry_type, "user");

        // EOF
        assert!(stream.next_entry()?.is_none());

        Ok(())
    }

    #[test]
    fn test_read_log_file() -> Result<()> {
        let file = create_test_log()?;
        let entries = read_log_file(file.path())?;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "queue-operation");
        assert_eq!(entries[1].entry_type, "user");

        Ok(())
    }

    #[test]
    fn test_empty_file() -> Result<()> {
        let file = NamedTempFile::new()?;
        let entries = read_log_file(file.path())?;
        assert_eq!(entries.len(), 0);
        Ok(())
    }

    #[test]
    fn test_skip_empty_lines() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2025-11-09T01:13:40.943Z"}}"#
        )?;
        writeln!(file)?; // Empty line
        writeln!(file)?; // Another empty line
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2025-11-09T01:14:00.358Z"}}"#
        )?;
        file.flush()?;

        let entries = read_log_file(file.path())?;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_type, "user");
        assert_eq!(entries[1].entry_type, "assistant");

        Ok(())
    }
}
