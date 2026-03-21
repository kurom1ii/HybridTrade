use std::{future::Future, pin::Pin, sync::Arc, time::Instant};

use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use futures_util::future::join_all;
use reqwest::Client;
use tokio::sync::mpsc;
use tracing::warn;

use crate::{agents::models::ChatStreamEvent, config::ProviderConfig};

use super::{
    super::{
        capabilities::CapabilityCatalog,
        hub::ProviderKind,
        protocols::call_provider,
    },
    models::{
        SpawnTeamMemberView, SpawnTeamReport, SpawnTeamRequest, SpawnTeamResult,
        SubagentFindings, SubagentStatus,
    },
    prompt::{build_kuromi_brief, build_subagent_system_prompt, build_subagent_task_message},
};

#[derive(Clone)]
pub(crate) struct TeamOrchestrator {
    client: Client,
    capabilities: CapabilityCatalog,
    provider: ProviderKind,
    provider_config: ProviderConfig,
    api_key: Option<String>,
    stream_sender: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
}

impl TeamOrchestrator {
    pub(in crate::agents::providers) fn new(
        client: Client,
        capabilities: CapabilityCatalog,
        provider: ProviderKind,
        provider_config: ProviderConfig,
        api_key: Option<String>,
        stream_sender: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
    ) -> Self {
        Self {
            client,
            capabilities,
            provider,
            provider_config,
            api_key,
            stream_sender,
        }
    }

    fn emit(&self, event: ChatStreamEvent) {
        if let Some(tx) = &self.stream_sender {
            let _ = tx.send(event);
        }
    }

    pub(crate) fn execute(
        &self,
        request: SpawnTeamRequest,
        context_preview: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<SpawnTeamResult>> + Send + '_>> {
        Box::pin(async move {
            let start = Instant::now();
            let request = request.validate()?;

            // ── Phase 1: Setup ──
            let member_names: Vec<String> =
                request.members.iter().map(|m| m.name.clone()).collect();
            let roster: Vec<String> = request
                .members
                .iter()
                .map(|m| format!("{} ({})", m.name, m.responsibility))
                .collect();

            self.emit(ChatStreamEvent::TeamStarted {
                session_id: String::new(),
                mission: request.mission.clone(),
                members: member_names.clone(),
            });

            // ── Phase 2: Parallel subagent execution ──
            let blackboard: Arc<DashMap<String, SubagentFindings>> = Arc::new(DashMap::new());
            let mut handles = Vec::with_capacity(request.members.len());

            for member in request.members.iter().cloned() {
                let client = self.client.clone();
                let capabilities = self.capabilities.clone();
                let provider = self.provider;
                let provider_config = self.provider_config.clone();
                let api_key = self.api_key.clone();
                let stream_sender = self.stream_sender.clone();
                let blackboard = Arc::clone(&blackboard);
                let request_clone = request.clone();
                let roster = roster.clone();
                let context_preview = context_preview.clone();

                let handle = tokio::spawn(async move {
                    // Emit member started
                    if let Some(tx) = &stream_sender {
                        let _ = tx.send(ChatStreamEvent::TeamMemberStarted {
                            member: member.name.clone(),
                        });
                    }

                    // Bootstrap isolated ToolRuntime
                    let mut runtime = capabilities
                        .tool_runtime_for_isolated(&[], context_preview.clone())
                        .await;
                    runtime.remove_definition("spawn_team");
                    if let Some(tx) = &stream_sender {
                        runtime.set_stream(tx.clone(), member.name.clone());
                    }

                    let system_prompt = build_subagent_system_prompt(
                        &request_clone,
                        &member.name,
                        &member.responsibility,
                        member.instructions.as_deref(),
                        &roster,
                    );

                    let user_message = build_subagent_task_message(
                        &request_clone,
                        &member.name,
                        &member.responsibility,
                        member.instructions.as_deref(),
                        context_preview.as_deref(),
                    );

                    runtime.prepare_turn(&[], context_preview);

                    let result = call_provider(
                        &client,
                        provider,
                        &provider_config,
                        api_key.as_deref(),
                        &system_prompt,
                        &[],
                        &user_message,
                        Some(provider_config.max_tokens),
                        Some(provider_config.temperature),
                        &mut runtime,
                        stream_sender.as_ref(),
                    )
                    .await;

                    let tool_calls = runtime.tool_calls().to_vec();
                    let completed_at = Utc::now().to_rfc3339();

                    let findings = match result {
                        Ok(content) => {
                            // Emit member response
                            if let Some(tx) = &stream_sender {
                                let _ = tx.send(ChatStreamEvent::TeamMemberResponse {
                                    member: member.name.clone(),
                                    round: 1,
                                    content: truncate_content(&content, 300),
                                    tool_calls: tool_calls.clone(),
                                });
                            }

                            SubagentFindings {
                                member: member.name.clone(),
                                responsibility: member.responsibility.clone(),
                                content,
                                tool_calls,
                                status: SubagentStatus::Completed,
                                completed_at,
                            }
                        }
                        Err(err) => {
                            let error_msg = format!("{err:#}");
                            warn!(
                                member = %member.name,
                                error = %error_msg,
                                "subagent failed"
                            );

                            SubagentFindings {
                                member: member.name.clone(),
                                responsibility: member.responsibility.clone(),
                                content: format!("Subagent thất bại: {error_msg}"),
                                tool_calls,
                                status: SubagentStatus::Failed(error_msg),
                                completed_at,
                            }
                        }
                    };

                    blackboard.insert(member.name.clone(), findings);
                });

                handles.push(handle);
            }

            // Wait for all subagents to complete
            let results = join_all(handles).await;
            for result in results {
                if let Err(err) = result {
                    warn!(error = %err, "subagent task panicked");
                }
            }

            // ── Phase 3: Read blackboard, build results ──
            let findings: Vec<SubagentFindings> = member_names
                .iter()
                .filter_map(|name| blackboard.remove(name).map(|(_, v)| v))
                .collect();

            let reports: Vec<SpawnTeamReport> = findings
                .iter()
                .map(|f| SpawnTeamReport {
                    member: f.member.clone(),
                    responsibility: f.responsibility.clone(),
                    report: f.content.clone(),
                })
                .collect();

            let kuromi_brief = build_kuromi_brief(&request, &findings);

            let members = request
                .members
                .iter()
                .map(SpawnTeamMemberView::from)
                .collect::<Vec<_>>();

            self.emit(ChatStreamEvent::TeamCompleted);

            let duration_ms = start.elapsed().as_millis() as u64;

            Ok(SpawnTeamResult {
                ok: true,
                mission: request.mission,
                briefing: request.briefing,
                duration_ms,
                provider: self.provider.name().to_string(),
                model: self.provider_config.model.clone(),
                members,
                reports,
                kuromi_brief,
            })
        })
    }
}

fn truncate_content(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>() + "..."
}
