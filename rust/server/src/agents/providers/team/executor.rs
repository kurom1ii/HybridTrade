use std::{future::Future, pin::Pin};

use anyhow::{Context, Result};
use futures_util::future::join_all;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::{
    agents::{
        models::ChatStreamEvent,
        tool_runtime::runtime::ToolRuntime,
    },
    config::ProviderConfig,
};

use super::{
    super::{
        capabilities::{ActiveSkill, CapabilityCatalog},
        hub::ProviderKind,
        protocols::call_provider,
    },
    models::{
        SpawnTeamMemberSpec, SpawnTeamMemberView, SpawnTeamReport, SpawnTeamRequest,
        SpawnTeamResult, SpawnTeamTranscriptEntry, TeamRuntimeContext,
    },
    prompt::{
        build_kuromi_brief, build_round_message, build_runtime_context_preview,
        build_subagent_system_prompt,
    },
};

#[derive(Clone)]
pub(crate) struct TeamOrchestrator {
    client: Client,
    capabilities: CapabilityCatalog,
    provider: ProviderKind,
    provider_config: ProviderConfig,
    light_config: Option<ProviderConfig>,
    api_key: Option<String>,
    active_skills: Vec<ActiveSkill>,
    stream_sender: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
}

struct TeamMemberState {
    member: SpawnTeamMemberSpec,
    runtime: ToolRuntime,
}

impl TeamOrchestrator {
    pub(in crate::agents::providers) fn new(
        client: Client,
        capabilities: CapabilityCatalog,
        provider: ProviderKind,
        provider_config: ProviderConfig,
        api_key: Option<String>,
        active_skills: Vec<ActiveSkill>,
        stream_sender: Option<mpsc::UnboundedSender<ChatStreamEvent>>,
    ) -> Self {
        let light_config = provider_config.light_config();
        Self {
            client,
            capabilities,
            provider,
            provider_config,
            light_config,
            api_key,
            active_skills,
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
        turn_context: TeamRuntimeContext,
    ) -> Pin<Box<dyn Future<Output = Result<SpawnTeamResult>> + Send + '_>> {
        Box::pin(async move {
            let request = request.validate()?;

            self.emit(ChatStreamEvent::TeamStarted {
                mission: request.mission.clone(),
                members: request.members.iter().map(|m| m.name.clone()).collect(),
            });

            // Mỗi subagent được cấp một runtime đầy đủ riêng biệt (native + MCP)
            // với Chrome DevTools chạy `--isolated` để có browser instance độc lập.
            let mut member_states = Vec::with_capacity(request.members.len());
            for member in request.members.iter().cloned() {
                let mut runtime = self
                    .capabilities
                    .tool_runtime_for_isolated(&[], turn_context.context_preview.clone())
                    .await;
                runtime.remove_definition("spawn_team");
                if let Some(tx) = &self.stream_sender {
                    runtime.set_stream(tx.clone(), member.name.clone());
                }
                member_states.push(TeamMemberState { member, runtime });
            }

            let mut transcript = Vec::new();
            for round_index in 0..request.rounds {
                let is_last_round = round_index + 1 == request.rounds;
                let round_config = if is_last_round {
                    &self.provider_config
                } else {
                    self.light_config.as_ref().unwrap_or(&self.provider_config)
                };
                let round_label = if is_last_round { "execute" } else { "discuss" };

                self.emit(ChatStreamEvent::TeamRound {
                    round: round_index + 1,
                    total: request.rounds,
                    phase: round_label.to_string(),
                });

                let runtime_context = build_runtime_context_preview(
                    &request,
                    &turn_context.history,
                    turn_context.context_preview.as_deref(),
                    &transcript,
                );
                let history_snapshot = turn_context.history.clone();
                let transcript_snapshot = transcript.clone();
                let request_snapshot = request.clone();

                let round_results = join_all(member_states.into_iter().map(|mut state| {
                    let runtime_context = runtime_context.clone();
                    let history_snapshot = history_snapshot.clone();
                    let transcript_snapshot = transcript_snapshot.clone();
                    let request_snapshot = request_snapshot.clone();
                    let active_skills = self.active_skills.clone();
                    let round_config = round_config.clone();
                    let is_final = is_last_round;

                    async move {
                        state.runtime.prepare_turn(&[], runtime_context);

                        let system_prompt = build_subagent_system_prompt(
                            &request_snapshot,
                            &state.member.name,
                            &state.member.responsibility,
                            state.member.instructions.as_deref(),
                        );
                        let message = build_round_message(
                            &request_snapshot,
                            round_index + 1,
                            request_snapshot.rounds,
                            &history_snapshot,
                            &transcript_snapshot,
                            &active_skills,
                        );

                        let content = call_provider(
                            &self.client,
                            self.provider,
                            &round_config,
                            self.api_key.as_deref(),
                            &system_prompt,
                            &[],
                            &message,
                            Some(if is_final {
                                round_config.max_tokens
                            } else {
                                round_config.max_tokens.min(900)
                            }),
                            Some(round_config.temperature),
                            &mut state.runtime,
                        )
                        .await
                        .with_context(|| {
                            format!(
                                "subagent `{}` thất bại ở round {}",
                                state.member.name,
                                round_index + 1
                            )
                        })?;

                        let speaker = state.member.name.clone();
                        let responsibility = state.member.responsibility.clone();
                        let member_tool_calls = state.runtime.tool_calls().to_vec();

                        Ok::<_, anyhow::Error>((
                            state,
                            SpawnTeamTranscriptEntry {
                                round: round_index + 1,
                                speaker,
                                responsibility,
                                content: content.trim().to_string(),
                                tool_calls: member_tool_calls,
                            },
                        ))
                    }
                }))
                .await;

                let mut next_member_states = Vec::with_capacity(round_results.len());
                let mut round_entries = Vec::with_capacity(round_results.len());

                for result in round_results {
                    let (state, entry) = result?;
                    self.emit(ChatStreamEvent::TeamMemberResponse {
                        member: entry.speaker.clone(),
                        round: entry.round,
                        content: truncate_content(&entry.content, 300),
                        tool_calls: entry.tool_calls.clone(),
                    });
                    next_member_states.push(state);
                    round_entries.push(entry);
                }

                transcript.extend(round_entries);
                member_states = next_member_states;
            }

            self.emit(ChatStreamEvent::TeamCompleted);

            let members = request
                .members
                .iter()
                .map(SpawnTeamMemberView::from)
                .collect::<Vec<_>>();
            let reports = build_reports(&request, &transcript);
            let kuromi_brief = build_kuromi_brief(&request, &reports);

            Ok(SpawnTeamResult {
                ok: true,
                mission: request.mission,
                briefing: request.briefing,
                rounds_completed: request.rounds,
                provider: self.provider.name().to_string(),
                model: self.provider_config.model.clone(),
                members,
                transcript,
                reports,
                kuromi_brief,
            })
        })
    }
}

fn build_reports(
    request: &SpawnTeamRequest,
    transcript: &[SpawnTeamTranscriptEntry],
) -> Vec<SpawnTeamReport> {
    request
        .members
        .iter()
        .map(|member| {
            let report = transcript
                .iter()
                .rev()
                .find(|entry| entry.speaker.eq_ignore_ascii_case(&member.name))
                .map(|entry| entry.content.clone())
                .unwrap_or_else(|| "Không có phản hồi nào được ghi nhận.".to_string());

            SpawnTeamReport {
                member: member.name.clone(),
                responsibility: member.responsibility.clone(),
                report,
            }
        })
        .collect()
}

fn truncate_content(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>() + "..."
}
