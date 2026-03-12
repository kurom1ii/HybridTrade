use std::{path::PathBuf, process::Stdio, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use tokio::{fs, io::AsyncWriteExt, process::Command, time::timeout};

use crate::agents::providers::team::{SpawnTeamRequest, TeamRuntimeContext};
use crate::config::NativeToolConfig;

use super::{
    runtime::{ToolDefinition, ToolExecutor, ToolRuntime},
    utils::{
        collapse_whitespace, collect_keywords, count_keyword_hits, ensure_path_is_within_workspace,
        extract_candidate_numbers, extract_domain, find_timeframes, normalize_path_for_workspace,
        optional_bool_arg, optional_raw_string_arg, optional_string_arg, optional_usize_arg,
        required_raw_string_arg, required_string_arg, resolve_requested_timeout, string_array_arg,
        truncate_chars, MAX_TOOL_OUTPUT_CHARS,
    },
};

#[derive(Debug, Clone, Copy)]
pub(super) enum NativeToolKind {
    ExtractSignals,
    MemoryLookup,
    SummarizeSources,
    Read,
    Write,
    Exec,
    Bash,
    SpawnTeam,
}

impl ToolRuntime {
    pub(super) fn register_native_tool(&mut self, config: NativeToolConfig) {
        let Some(kind) = native_tool_kind(&config.name) else {
            self.initialization_warnings.push(format!(
                "native tool `{}` chưa có executor Rust tương ứng, nên bị bỏ qua",
                config.name
            ));
            return;
        };

        let definition = ToolDefinition {
            name: config.name.clone(),
            description: native_tool_description(kind).to_string(),
            input_schema: native_tool_schema(kind),
            source_label: format!("native:{}", config.name),
            executor: ToolExecutor::Native {
                kind,
                timeout: Duration::from_millis(config.timeout_ms.max(1_000)),
            },
        };

        self.insert_definition(definition);
    }

    pub(super) async fn execute_native_tool(
        &self,
        kind: NativeToolKind,
        tool_timeout: Duration,
        arguments: Value,
    ) -> Result<Value> {
        match timeout(tool_timeout, async {
            match kind {
                NativeToolKind::ExtractSignals => self.extract_signals(arguments),
                NativeToolKind::MemoryLookup => self.memory_lookup(arguments),
                NativeToolKind::SummarizeSources => self.summarize_sources(arguments),
                NativeToolKind::Read => self.read_path(arguments).await,
                NativeToolKind::Write => self.write_path(arguments).await,
                NativeToolKind::Exec => self.exec_command(arguments, tool_timeout).await,
                NativeToolKind::Bash => self.run_bash(arguments, tool_timeout).await,
                NativeToolKind::SpawnTeam => self.spawn_team(arguments).await,
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => bail!(
                "native tool `{}` vượt quá timeout {}ms",
                native_tool_name(kind),
                tool_timeout.as_millis()
            ),
        }
    }

    fn extract_signals(&self, arguments: Value) -> Result<Value> {
        let text = required_string_arg(&arguments, "text")?;
        let lowered = text.to_ascii_lowercase();

        let bullish_hits = count_keyword_hits(&lowered, &["bull", "bullish", "breakout", "long"]);
        let bearish_hits = count_keyword_hits(&lowered, &["bear", "bearish", "breakdown", "short"]);

        let bias = match bullish_hits.cmp(&bearish_hits) {
            std::cmp::Ordering::Greater => "bullish",
            std::cmp::Ordering::Less => "bearish",
            std::cmp::Ordering::Equal => "neutral",
        };

        Ok(json!({
            "ok": true,
            "bias": bias,
            "timeframes": find_timeframes(&lowered),
            "levels": extract_candidate_numbers(&text),
            "keywords": collect_keywords(&lowered),
        }))
    }

    fn memory_lookup(&self, arguments: Value) -> Result<Value> {
        let query = optional_string_arg(&arguments, "query").unwrap_or_default();
        let lowered_query = query.to_ascii_lowercase();
        let tokens = lowered_query
            .split_whitespace()
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();

        let mut corpus = Vec::new();
        if let Some(context_preview) = self.context_preview.as_deref() {
            for line in context_preview
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                corpus.push(format!("context: {line}"));
            }
        }

        for turn in &self.history {
            corpus.push(format!(
                "{}: {}",
                turn.role,
                collapse_whitespace(&turn.content)
            ));
        }

        let matches = corpus
            .into_iter()
            .filter(|line| {
                if tokens.is_empty() {
                    return true;
                }

                let lowered = line.to_ascii_lowercase();
                tokens.iter().all(|token| lowered.contains(token))
            })
            .take(8)
            .collect::<Vec<_>>();

        Ok(json!({
            "ok": true,
            "query": query,
            "matches": matches,
            "history_turns": self.history.len(),
            "has_backend_context": self.context_preview.is_some(),
        }))
    }

    fn summarize_sources(&self, arguments: Value) -> Result<Value> {
        let mut items = string_array_arg(&arguments, "urls");
        if items.is_empty() {
            items = string_array_arg(&arguments, "items");
        }
        if items.is_empty() {
            if let Some(text) = optional_string_arg(&arguments, "text") {
                items = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect();
            }
        }

        if items.is_empty() {
            bail!("summarize_sources cần `urls`, `items` hoặc `text`");
        }

        let mut domains = Vec::new();
        for item in &items {
            if let Some(domain) = extract_domain(item) {
                if !domains.iter().any(|entry| entry == &domain) {
                    domains.push(domain);
                }
            }
        }

        Ok(json!({
            "ok": true,
            "count": items.len(),
            "domains": domains,
            "items": items.into_iter().take(10).collect::<Vec<_>>(),
        }))
    }

    pub(super) async fn read_path(&self, arguments: Value) -> Result<Value> {
        let requested_path = required_string_arg(&arguments, "path")?;
        let path = self.resolve_workspace_path(&requested_path)?;
        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("không thể đọc metadata `{}`", path.display()))?;

        if metadata.is_dir() {
            bail!(
                "`{}` là thư mục, tool `read` chỉ hỗ trợ file",
                path.display()
            );
        }

        let bytes = fs::read(&path)
            .await
            .with_context(|| format!("không thể đọc file `{}`", path.display()))?;
        let lossy_utf8 = std::str::from_utf8(&bytes).is_err();
        let content = String::from_utf8_lossy(&bytes).into_owned();
        let start_line = optional_usize_arg(&arguments, "start_line")
            .unwrap_or(1)
            .max(1);
        let line_count = optional_usize_arg(&arguments, "line_count")
            .unwrap_or(200)
            .clamp(1, 2_000);
        let max_chars = optional_usize_arg(&arguments, "max_chars")
            .unwrap_or(6_000)
            .clamp(1, MAX_TOOL_OUTPUT_CHARS);

        let lines = content.lines().collect::<Vec<_>>();
        let start_index = start_line.saturating_sub(1).min(lines.len());
        let end_index = (start_index + line_count).min(lines.len());
        let mut snippet = lines[start_index..end_index].join("\n");
        let truncated = end_index < lines.len() || snippet.chars().count() > max_chars;

        if snippet.chars().count() > max_chars {
            snippet = truncate_chars(&snippet, max_chars);
        }

        Ok(json!({
            "ok": true,
            "path": requested_path,
            "resolved_path": path.display().to_string(),
            "size_bytes": metadata.len(),
            "total_lines": lines.len(),
            "start_line": if lines.is_empty() { 0 } else { start_index + 1 },
            "end_line": end_index,
            "truncated": truncated,
            "lossy_utf8": lossy_utf8,
            "content": snippet,
        }))
    }

    pub(super) async fn write_path(&self, arguments: Value) -> Result<Value> {
        let requested_path = required_string_arg(&arguments, "path")?;
        let content = required_raw_string_arg(&arguments, "content")?;
        let append = optional_bool_arg(&arguments, "append").unwrap_or(false);
        let create_parent_dirs =
            optional_bool_arg(&arguments, "create_parent_dirs").unwrap_or(true);
        let path = self.resolve_workspace_path(&requested_path)?;

        let Some(parent) = path.parent() else {
            bail!("không thể xác định thư mục cha cho `{}`", path.display());
        };

        if !parent.exists() {
            if create_parent_dirs {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("không thể tạo thư mục cha `{}`", parent.display()))?;
            } else {
                bail!("thư mục cha `{}` chưa tồn tại", parent.display());
            }
        }

        let existed = path.exists();
        if append {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .with_context(|| format!("không thể mở file `{}` để append", path.display()))?;
            file.write_all(content.as_bytes())
                .await
                .with_context(|| format!("không thể append vào `{}`", path.display()))?;
            file.flush().await.ok();
        } else {
            fs::write(&path, content.as_bytes())
                .await
                .with_context(|| format!("không thể ghi file `{}`", path.display()))?;
        }

        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("không thể đọc metadata `{}` sau khi ghi", path.display()))?;

        Ok(json!({
            "ok": true,
            "path": requested_path,
            "resolved_path": path.display().to_string(),
            "mode": if append { "append" } else { "write" },
            "created": !existed,
            "bytes_written": content.len(),
            "size_bytes": metadata.len(),
        }))
    }

    pub(super) async fn exec_command(
        &self,
        arguments: Value,
        tool_timeout: Duration,
    ) -> Result<Value> {
        let command = required_string_arg(&arguments, "command")?;
        let args = string_array_arg(&arguments, "args");
        let cwd = self.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
        let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

        let mut process = Command::new(&command);
        process
            .args(&args)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = match timeout(effective_timeout, process.output()).await {
            Ok(result) => result.with_context(|| format!("không thể chạy lệnh `{command}`"))?,
            Err(_) => {
                bail!(
                    "exec `{command}` vượt quá timeout {}ms",
                    effective_timeout.as_millis()
                )
            }
        };

        Ok(json!({
            "ok": output.status.success(),
            "command": command,
            "args": args,
            "cwd": cwd.display().to_string(),
            "timeout_ms": effective_timeout.as_millis(),
            "exit_code": output.status.code(),
            "success": output.status.success(),
            "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
            "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
        }))
    }

    async fn run_bash(&self, arguments: Value, tool_timeout: Duration) -> Result<Value> {
        let script = optional_raw_string_arg(&arguments, "script")
            .or_else(|| optional_raw_string_arg(&arguments, "command"))
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("thiếu tham số string bắt buộc `script`"))?;
        let cwd = self.resolve_command_cwd(optional_string_arg(&arguments, "cwd").as_deref())?;
        let effective_timeout = resolve_requested_timeout(&arguments, tool_timeout);

        let mut process = Command::new("bash");
        process
            .arg("-lc")
            .arg(&script)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let output = match timeout(effective_timeout, process.output()).await {
            Ok(result) => result.context("không thể chạy bash process")?,
            Err(_) => {
                bail!("bash vượt quá timeout {}ms", effective_timeout.as_millis())
            }
        };

        Ok(json!({
            "ok": output.status.success(),
            "script": script,
            "cwd": cwd.display().to_string(),
            "timeout_ms": effective_timeout.as_millis(),
            "exit_code": output.status.code(),
            "success": output.status.success(),
            "stdout": truncate_chars(&String::from_utf8_lossy(&output.stdout), 3_000),
            "stderr": truncate_chars(&String::from_utf8_lossy(&output.stderr), 3_000),
        }))
    }

    async fn spawn_team(&self, arguments: Value) -> Result<Value> {
        let request: SpawnTeamRequest =
            serde_json::from_value(arguments).context("payload của spawn_team không hợp lệ")?;
        let Some(team_orchestrator) = self.team_orchestrator.as_ref() else {
            bail!("spawn_team chưa được gắn team orchestrator ở runtime hiện tại");
        };

        let output = team_orchestrator
            .execute(
                request,
                TeamRuntimeContext {
                    history: self.history.clone(),
                    context_preview: self.context_preview.clone(),
                },
            )
            .await?;

        serde_json::to_value(output).context("không thể serialize kết quả spawn_team")
    }

    fn resolve_workspace_path(&self, requested_path: &str) -> Result<PathBuf> {
        let candidate = PathBuf::from(requested_path);
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            self.workspace_root.join(candidate)
        };
        let normalized = normalize_path_for_workspace(&candidate)?;
        ensure_path_is_within_workspace(&normalized, &self.workspace_root)?;
        Ok(normalized)
    }

    fn resolve_command_cwd(&self, requested_cwd: Option<&str>) -> Result<PathBuf> {
        let cwd = match requested_cwd {
            Some(value) => self.resolve_workspace_path(value)?,
            None => self.workspace_root.clone(),
        };

        if !cwd.is_dir() {
            bail!("cwd `{}` không phải thư mục", cwd.display());
        }

        Ok(cwd)
    }
}

pub(super) fn native_tool_kind(name: &str) -> Option<NativeToolKind> {
    match name.trim().to_ascii_lowercase().as_str() {
        "extract_signals" => Some(NativeToolKind::ExtractSignals),
        "memory_lookup" => Some(NativeToolKind::MemoryLookup),
        "summarize_sources" => Some(NativeToolKind::SummarizeSources),
        "read" => Some(NativeToolKind::Read),
        "write" => Some(NativeToolKind::Write),
        "exec" => Some(NativeToolKind::Exec),
        "bash" => Some(NativeToolKind::Bash),
        "spawn_team" => Some(NativeToolKind::SpawnTeam),
        _ => None,
    }
}

fn native_tool_name(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::ExtractSignals => "extract_signals",
        NativeToolKind::MemoryLookup => "memory_lookup",
        NativeToolKind::SummarizeSources => "summarize_sources",
        NativeToolKind::Read => "read",
        NativeToolKind::Write => "write",
        NativeToolKind::Exec => "exec",
        NativeToolKind::Bash => "bash",
        NativeToolKind::SpawnTeam => "spawn_team",
    }
}

pub(super) fn native_tool_description(kind: NativeToolKind) -> &'static str {
    match kind {
        NativeToolKind::ExtractSignals => {
            "Rút bias, timeframe, keywords và các mức giá ứng viên từ raw text kỹ thuật."
        }
        NativeToolKind::MemoryLookup => {
            "Tra nhanh backend context preview và lịch sử chat hiện tại theo từ khoá."
        }
        NativeToolKind::SummarizeSources => {
            "Tóm tắt danh sách URL hoặc item nguồn thành số lượng và domain chính."
        }
        NativeToolKind::Read => {
            "Đọc file trong workspace của backend với cửa sổ dòng và giới hạn ký tự."
        }
        NativeToolKind::Write => {
            "Ghi hoặc append nội dung text vào file trong workspace của backend."
        }
        NativeToolKind::Exec => "Chạy một executable trực tiếp trong workspace mà không qua shell.",
        NativeToolKind::Bash => {
            "Chạy một lệnh bash ngắn trong workspace để debug hoặc thao tác nhanh."
        }
        NativeToolKind::SpawnTeam => {
            "Spawn một team subagent runtime-only, cho họ trao đổi qua transcript chung rồi trả báo cáo về cho Kuromi."
        }
    }
}

fn native_tool_schema(kind: NativeToolKind) -> Value {
    match kind {
        NativeToolKind::ExtractSignals => json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Đoạn text kỹ thuật cần rút tín hiệu"
                }
            },
            "required": ["text"],
            "additionalProperties": false,
        }),
        NativeToolKind::MemoryLookup => json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Từ khoá cần tra trong backend context và history"
                }
            },
            "additionalProperties": false,
        }),
        NativeToolKind::SummarizeSources => json!({
            "type": "object",
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách URL nguồn"
                },
                "items": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách item nguồn bất kỳ"
                },
                "text": {
                    "type": "string",
                    "description": "Raw text chứa danh sách nguồn, mỗi dòng một item"
                }
            },
            "additionalProperties": false,
        }),
        NativeToolKind::Read => json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path file cần đọc, relative theo workspace tool"
                },
                "start_line": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Dòng bắt đầu, mặc định 1"
                },
                "line_count": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Số dòng cần đọc, mặc định 200"
                },
                "max_chars": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Giới hạn ký tự trả về"
                }
            },
            "required": ["path"],
            "additionalProperties": false,
        }),
        NativeToolKind::Write => json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path file cần ghi, relative theo workspace tool"
                },
                "content": {
                    "type": "string",
                    "description": "Nội dung text cần ghi"
                },
                "append": {
                    "type": "boolean",
                    "description": "Nếu true sẽ append thay vì overwrite"
                },
                "create_parent_dirs": {
                    "type": "boolean",
                    "description": "Nếu true sẽ tự tạo thư mục cha còn thiếu"
                }
            },
            "required": ["path", "content"],
            "additionalProperties": false,
        }),
        NativeToolKind::Exec => json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Tên executable cần chạy"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Danh sách argument truyền vào executable"
                },
                "cwd": {
                    "type": "string",
                    "description": "Thư mục làm việc, relative theo workspace tool"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Timeout mong muốn, không vượt quá timeout của tool"
                }
            },
            "required": ["command"],
            "additionalProperties": false,
        }),
        NativeToolKind::Bash => json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "Đoạn lệnh bash cần chạy"
                },
                "cwd": {
                    "type": "string",
                    "description": "Thư mục làm việc, relative theo workspace tool"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Timeout mong muốn, không vượt quá timeout của tool"
                }
            },
            "required": ["script"],
            "additionalProperties": false,
        }),
        NativeToolKind::SpawnTeam => json!({
            "type": "object",
            "properties": {
                "mission": {
                    "type": "string",
                    "description": "Mục tiêu chung mà team dynamic cần giải quyết"
                },
                "briefing": {
                    "type": "string",
                    "description": "Bổ sung ngắn từ Kuromi để các subagent bám vào"
                },
                "rounds": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 4,
                    "description": "Số vòng thảo luận cho team, mặc định 2"
                },
                "report_instruction": {
                    "type": "string",
                    "description": "Định dạng hoặc yêu cầu riêng cho báo cáo cuối của team"
                },
                "members": {
                    "type": "array",
                    "description": "Danh sách subagent cần spawn cho mission này",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Tên hiển thị của subagent"
                            },
                            "responsibility": {
                                "type": "string",
                                "description": "Trọng trách hoặc góc phân tích chính của subagent"
                            },
                            "instructions": {
                                "type": "string",
                                "description": "Chỉ dẫn bổ sung riêng cho subagent này"
                            }
                        },
                        "required": ["name", "responsibility"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["mission", "members"],
            "additionalProperties": false,
        }),
    }
}
