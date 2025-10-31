use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured event schema for R2 sync daemon logging
///
/// Design: Unified JSONL event stream per daemon replacing fragmented text logs.
/// Each event is a single-line JSON object with timestamp and daemon context.
///
/// Benefits:
/// - Machine-parseable by default (no regex/emoji detection)
/// - Performance: read last line for most recent event
/// - Rich queries with jq (error analysis, transfer stats, patterns)
/// - Type-safe parsing in Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SyncEvent {
    /// Daemon process started (launchd or manual)
    DaemonStart {
        timestamp: DateTime<Utc>,
        daemon: String,
        pid: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        config: Option<HashMap<String, String>>,
    },

    /// Daemon process stopped (graceful or crash)
    DaemonStop {
        timestamp: DateTime<Utc>,
        daemon: String,
        reason: String, // "graceful" | "signal" | "error"
    },

    /// File change detected (triggers debounce timer)
    FileChange {
        timestamp: DateTime<Utc>,
        daemon: String,
        path: String,
        debounce_ms: u64,
    },

    /// Sync operation started
    SyncStart {
        timestamp: DateTime<Utc>,
        daemon: String,
        trigger: String, // "auto" | "manual" | "cron"
    },

    /// Sync operation completed (success or failure)
    SyncComplete {
        timestamp: DateTime<Utc>,
        daemon: String,
        success: bool,
        files_transferred: usize,
        bytes_transferred: u64,
        duration_ms: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        transfer_rate_bps: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
    },

    /// Sync error occurred
    SyncError {
        timestamp: DateTime<Utc>,
        daemon: String,
        error_type: String,  // "network" | "permission" | "s3" | "rclone" | "timeout"
        error_message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<HashMap<String, String>>,
    },
}

impl SyncEvent {
    /// Get timestamp from any event variant
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            SyncEvent::DaemonStart { timestamp, .. } => timestamp,
            SyncEvent::DaemonStop { timestamp, .. } => timestamp,
            SyncEvent::FileChange { timestamp, .. } => timestamp,
            SyncEvent::SyncStart { timestamp, .. } => timestamp,
            SyncEvent::SyncComplete { timestamp, .. } => timestamp,
            SyncEvent::SyncError { timestamp, .. } => timestamp,
        }
    }

    /// Get daemon name from any event variant
    pub fn daemon(&self) -> &str {
        match self {
            SyncEvent::DaemonStart { daemon, .. } => daemon,
            SyncEvent::DaemonStop { daemon, .. } => daemon,
            SyncEvent::FileChange { daemon, .. } => daemon,
            SyncEvent::SyncStart { daemon, .. } => daemon,
            SyncEvent::SyncComplete { daemon, .. } => daemon,
            SyncEvent::SyncError { daemon, .. } => daemon,
        }
    }

    /// Check if this is a successful sync completion event
    pub fn is_successful_sync(&self) -> bool {
        matches!(self, SyncEvent::SyncComplete { success: true, .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_daemon_start() {
        let event = SyncEvent::DaemonStart {
            timestamp: Utc::now(),
            daemon: "daily".to_string(),
            pid: 12345,
            config: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""event":"daemon_start"#));
        assert!(json.contains(r#""daemon":"daily"#));
        assert!(json.contains(r#""pid":12345"#));
    }

    #[test]
    fn test_serialize_sync_complete() {
        let event = SyncEvent::SyncComplete {
            timestamp: Utc::now(),
            daemon: "daily".to_string(),
            success: true,
            files_transferred: 42,
            bytes_transferred: 1024000,
            duration_ms: 5000,
            transfer_rate_bps: Some(204800),
            error_message: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""event":"sync_complete"#));
        assert!(json.contains(r#""files_transferred":42"#));
        assert!(json.contains(r#""bytes_transferred":1024000"#));
    }

    #[test]
    fn test_deserialize_round_trip() {
        let original = SyncEvent::SyncError {
            timestamp: Utc::now(),
            daemon: "dispatch".to_string(),
            error_type: "network".to_string(),
            error_message: "connection timeout".to_string(),
            context: Some([("host".to_string(), "s3.amazonaws.com".to_string())].into()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SyncEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(original.daemon(), deserialized.daemon());
        assert!(matches!(deserialized, SyncEvent::SyncError { .. }));
    }

    #[test]
    fn test_event_helpers() {
        let event = SyncEvent::SyncComplete {
            timestamp: Utc::now(),
            daemon: "daily".to_string(),
            success: true,
            files_transferred: 10,
            bytes_transferred: 5000,
            duration_ms: 2000,
            transfer_rate_bps: None,
            error_message: None,
        };

        assert_eq!(event.daemon(), "daily");
        assert!(event.is_successful_sync());
    }
}
