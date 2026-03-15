use std::{future::Future, pin::Pin};

use anyhow::{Context, Result};
use reqwest::Client;
use tokio::sync::mpsc;

use crate::{agents::models::ChatStreamEvent, config::ProviderConfig};

use super::{
    super::{
        capabilities::{ActiveSkill, CapabilityCatalog},
        hub::ProviderKind,
        protocols::call_provider,
    },
    models::{
        SpawnTeamMemberSpec, SpawnTeamMemberView, SpawnTeamReport, SpawnTeamRequest,
        SpawnTeamResult, TeamRuntimeContext,
    },
    prompt::{
        build_directive_content, build_kuromi_brief, build_member_inbox,
        build_runtime_context_preview, build_subagent_system_prompt,
    },
    session::{TeamMessageKind, TeamSession},
};

use crate::agents::tool_runtime::runtime::ToolRuntime;

#[derive(Clone)]
pub(crate) struct TeamOrchestrator {
    client: Client,
    capabilities: CapabilityCatalog,
    provider: ProviderKind,
    provider_config: ProviderConfig,
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
        Self {
            client,
            capabilities,
            provider,
            provider_config,
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
            let mut session = TeamSession::new(None);

            // ── Phase 1: Setup ──
            let member_names: Vec<String> =
                request.members.iter().map(|m| m.name.clone()).collect();
            let roster: Vec<String> = request
                .members
                .iter()
                .map(|m| format!("{} ({})", m.name, m.responsibility))
                .collect();

            session.append(
                "system",
                "*",
                TeamMessageKind::System,
                &format!(
                    "Team spawned. Mission: {}. Members: [{}]",
                    request.mission,
                    roster.join(", ")
                ),
                vec![],
                None,
            );

            self.emit(ChatStreamEvent::TeamStarted {
                session_id: session.session_id().to_string(),
                mission: request.mission.clone(),
                members: member_names.clone(),
            });

            // Bootstrap isolated runtimes for each member
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

            // ── Phase 2: Exchange Cycles (sequential) ──
            for exchange in 0..request.rounds {
                let is_last = exchange + 1 == request.rounds;
                let phase_label = if is_last { "execute" } else { "discuss" };

                self.emit(ChatStreamEvent::TeamRound {
                    round: exchange + 1,
                    total: request.rounds,
                    phase: phase_label.to_string(),
                });

                // Leader sends directive (broadcast to all)
                let directive = build_directive_content(
                    &request,
                    exchange,
                    is_last,
                    &session,
                );
                let directive_msg = session.append(
                    "kuromi",
                    "*",
                    TeamMessageKind::Directive,
                    &directive,
                    vec![],
                    Some(serde_json::json!({ "exchange": exchange + 1 })),
                );
                let directive_seq = directive_msg.seq;
                let directive_session_id = session.session_id().to_string();

                self.emit(ChatStreamEvent::TeamDirective {
                    session_id: directive_session_id,
                    seq: directive_seq,
                    to: "*".to_string(),
                    content_preview: truncate_content(&directive, 200),
                });

                // Members respond SEQUENTIALLY (not parallel)
                let mut next_member_states = Vec::with_capacity(member_states.len());

                for mut state in member_states.into_iter() {
                    let runtime_context = build_runtime_context_preview(
                        &request,
                        &turn_context.history,
                        turn_context.context_preview.as_deref(),
                        &session,
                    );

                    state.runtime.prepare_turn(&[], runtime_context);

                    let system_prompt = build_subagent_system_prompt(
                        &request,
                        &state.member.name,
                        &state.member.responsibility,
                        state.member.instructions.as_deref(),
                        &roster,
                        session.session_id(),
                    );

                    let inbox = build_member_inbox(
                        &state.member.name,
                        &session,
                        &self.active_skills,
                    );

                    let config = &self.provider_config;
                    let content = call_provider(
                        &self.client,
                        self.provider,
                        config,
                        self.api_key.as_deref(),
                        &system_prompt,
                        &[],
                        &inbox,
                        Some(if is_last {
                            config.max_tokens
                        } else {
                            config.max_tokens.min(4096)
                        }),
                        Some(config.temperature),
                        &mut state.runtime,
                        None,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "subagent `{}` thất bại ở exchange {}",
                            state.member.name,
                            exchange + 1
                        )
                    })?;

                    let member_tool_calls = state.runtime.tool_calls().to_vec();

                    // Append response to session
                    session.append(
                        &state.member.name,
                        "kuromi",
                        TeamMessageKind::Response,
                        content.trim(),
                        member_tool_calls.clone(),
                        Some(serde_json::json!({ "exchange": exchange + 1 })),
                    );

                    self.emit(ChatStreamEvent::TeamMemberResponse {
                        member: state.member.name.clone(),
                        round: exchange + 1,
                        content: truncate_content(&content, 300),
                        tool_calls: member_tool_calls,
                    });

                    next_member_states.push(state);
                }

                member_states = next_member_states;
            }

            // ── Phase 3: Completion ──
            session.append(
                "system",
                "*",
                TeamMessageKind::System,
                "Team completed.",
                vec![],
                None,
            );

            self.emit(ChatStreamEvent::TeamCompleted);

            let members = request
                .members
                .iter()
                .map(SpawnTeamMemberView::from)
                .collect::<Vec<_>>();
            let reports = build_reports(&request, &session);
            let kuromi_brief = build_kuromi_brief(&request, &reports);

            Ok(SpawnTeamResult {
                ok: true,
                mission: request.mission,
                briefing: request.briefing,
                session_id: session.session_id().to_string(),
                log_path: session.log_path().to_string(),
                exchanges_completed: request.rounds,
                provider: self.provider.name().to_string(),
                model: self.provider_config.model.clone(),
                members,
                reports,
                kuromi_brief,
            })
        })
    }
}

fn build_reports(
    request: &SpawnTeamRequest,
    session: &TeamSession,
) -> Vec<SpawnTeamReport> {
    request
        .members
        .iter()
        .map(|member| {
            // Find the last response from this member
            let report = session
                .messages()
                .iter()
                .rev()
                .find(|msg| {
                    msg.from.eq_ignore_ascii_case(&member.name)
                        && msg.kind == TeamMessageKind::Response
                })
                .map(|msg| msg.content.clone())
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
