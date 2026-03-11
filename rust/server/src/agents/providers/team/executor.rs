use std::{future::Future, pin::Pin};

use anyhow::{Context, Result};
use reqwest::Client;

use crate::{agents::models::AgentRole, config::ProviderConfig};

use super::{
    super::{capabilities::CapabilityCatalog, hub::ProviderKind, protocols::call_provider},
    models::{
        SpawnTeamMemberView, SpawnTeamReport, SpawnTeamRequest, SpawnTeamResult,
        SpawnTeamTranscriptEntry, TeamRuntimeContext,
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
    api_key: Option<String>,
}

impl TeamOrchestrator {
    pub(in crate::agents::providers) fn new(
        client: Client,
        capabilities: CapabilityCatalog,
        provider: ProviderKind,
        provider_config: ProviderConfig,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client,
            capabilities,
            provider,
            provider_config,
            api_key,
        }
    }

    pub(crate) fn execute(
        &self,
        request: SpawnTeamRequest,
        turn_context: TeamRuntimeContext,
    ) -> Pin<Box<dyn Future<Output = Result<SpawnTeamResult>> + Send + '_>> {
        Box::pin(async move {
            let request = request.validate()?;
            let prompt_profile = self.capabilities.prompt_profile_for(AgentRole::Kuromi);

            let mut member_runtimes = Vec::with_capacity(request.members.len());
            for _ in &request.members {
                let mut runtime = self
                    .capabilities
                    .tool_runtime_for(
                        &[],
                        turn_context.context_preview.clone(),
                        self.client.clone(),
                    )
                    .await;
                runtime.remove_definition("spawn_team");
                member_runtimes.push(runtime);
            }

            let mut transcript = Vec::new();
            for round_index in 0..request.rounds {
                for (member_index, member) in request.members.iter().enumerate() {
                    let runtime_context = build_runtime_context_preview(
                        &request,
                        &turn_context.history,
                        turn_context.context_preview.as_deref(),
                        &transcript,
                    );

                    member_runtimes[member_index].prepare_turn(&[], runtime_context);

                    let system_prompt = build_subagent_system_prompt(
                        &prompt_profile.common_markdown,
                        &prompt_profile.agent_markdown,
                        &request,
                        &member.name,
                        &member.responsibility,
                        member.instructions.as_deref(),
                    );
                    let message = build_round_message(
                        &request,
                        round_index + 1,
                        request.rounds,
                        &turn_context.history,
                        &transcript,
                    );

                    let content = call_provider(
                        &self.client,
                        self.provider,
                        &self.provider_config,
                        self.api_key.as_deref(),
                        &system_prompt,
                        &[],
                        &message,
                        Some(self.provider_config.max_tokens.min(900)),
                        Some(self.provider_config.temperature),
                        &mut member_runtimes[member_index],
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "subagent `{}` thất bại ở round {}",
                            member.name,
                            round_index + 1
                        )
                    })?;

                    transcript.push(SpawnTeamTranscriptEntry {
                        round: round_index + 1,
                        speaker: member.name.clone(),
                        responsibility: member.responsibility.clone(),
                        content: content.trim().to_string(),
                    });
                }
            }

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
