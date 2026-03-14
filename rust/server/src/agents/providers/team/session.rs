use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;
use uuid::Uuid;

use crate::agents::models::DebugToolCall;

const DEFAULT_TEAM_CHAT_LOG_DIR: &str = "./logs/team-chats";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TeamMessage {
    pub(crate) ts: String,
    pub(crate) session_id: String,
    pub(crate) seq: usize,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) kind: TeamMessageKind,
    pub(crate) content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tool_calls: Vec<DebugToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TeamMessageKind {
    System,
    Directive,
    Response,
    Discussion,
}

pub(crate) struct TeamSession {
    session_id: String,
    log_path: PathBuf,
    messages: Vec<TeamMessage>,
    log_file: File,
    seq: AtomicUsize,
}

impl TeamSession {
    pub(crate) fn new(log_dir: Option<&str>) -> Self {
        let session_id = Uuid::new_v4().to_string()[..8].to_string();
        let dir = log_dir
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HYBRIDTRADE_TEAM_CHAT_LOG_DIR")
                    .ok()
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TEAM_CHAT_LOG_DIR));

        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("Failed to create team chat log dir {:?}: {}", dir, e);
        }

        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let filename = format!("{}_team-{}.jsonl", date_str, session_id);
        let log_path = dir.join(&filename);

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap_or_else(|e| {
                warn!("Failed to open team chat log {:?}: {}", log_path, e);
                // Fallback to /dev/null equivalent
                OpenOptions::new()
                    .write(true)
                    .open("/dev/null")
                    .expect("cannot open /dev/null")
            });

        Self {
            session_id,
            log_path,
            messages: Vec::new(),
            log_file,
            seq: AtomicUsize::new(0),
        }
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn log_path(&self) -> &str {
        self.log_path.to_str().unwrap_or("unknown")
    }

    pub(crate) fn messages(&self) -> &[TeamMessage] {
        &self.messages
    }

    fn next_seq(&self) -> usize {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub(crate) fn append(
        &mut self,
        from: &str,
        to: &str,
        kind: TeamMessageKind,
        content: &str,
        tool_calls: Vec<DebugToolCall>,
        meta: Option<Value>,
    ) -> &TeamMessage {
        let msg = TeamMessage {
            ts: Utc::now().to_rfc3339(),
            session_id: self.session_id.clone(),
            seq: self.next_seq(),
            from: from.to_string(),
            to: to.to_string(),
            kind,
            content: content.to_string(),
            tool_calls,
            meta,
        };

        // Write to JSONL file (best-effort)
        if let Ok(line) = serde_json::to_string(&msg) {
            if let Err(e) = writeln!(self.log_file, "{}", line) {
                warn!("Failed to write team chat log: {}", e);
            }
        }

        self.messages.push(msg);
        self.messages.last().unwrap()
    }

    /// Returns messages visible to a specific member:
    /// - All system messages
    /// - All broadcast messages (to = "*")
    /// - Messages addressed to this member (to = member_name)
    /// - Messages sent by this member (from = member_name)
    /// - All responses from other members (cross-pollination for sequential flow)
    pub(crate) fn messages_visible_for(&self, member_name: &str) -> Vec<&TeamMessage> {
        self.messages
            .iter()
            .filter(|msg| {
                msg.kind == TeamMessageKind::System
                    || msg.to == "*"
                    || msg.to.eq_ignore_ascii_case(member_name)
                    || msg.from.eq_ignore_ascii_case(member_name)
                    || msg.kind == TeamMessageKind::Response
                    || msg.kind == TeamMessageKind::Directive
            })
            .collect()
    }
}
