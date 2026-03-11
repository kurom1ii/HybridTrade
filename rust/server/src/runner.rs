use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::error;

use crate::{
    db,
    models::{AgentRole, Citation, FindingRow, InvestigationRow, SectionRow, SourceDocumentRow},
    AppState,
};

struct AnalysisSnapshot {
    pair_hint: Option<String>,
    bias: String,
    confidence: f64,
    levels: Vec<String>,
    contradiction: bool,
    summary: String,
}

pub fn spawn_investigation(state: Arc<AppState>, investigation_id: String) {
    tokio::spawn(async move {
        if let Err(error) = run_investigation(state.clone(), &investigation_id).await {
            error!(investigation_id, error = %error, "investigation failed");
            let _ = db::set_investigation_status(
                &state.db,
                &investigation_id,
                "failed",
                Some(&format!("Run that bai: {error}")),
            )
            .await;
            let _ = db::upsert_heartbeat(
                &state.db,
                "investigation",
                &investigation_id,
                "failed",
                state.config.heartbeat.agent_ttl_seconds,
                json!({ "error": error.to_string() }),
            )
            .await;
            let _ = publish_investigation_update(state, &investigation_id).await;
        }
    });
}

pub fn spawn_follow_up(
    state: Arc<AppState>,
    investigation_id: String,
    question: String,
    target_section_id: Option<String>,
) {
    tokio::spawn(async move {
        if let Err(error) = answer_follow_up(
            state,
            &investigation_id,
            &question,
            target_section_id.as_deref(),
        )
        .await
        {
            error!(investigation_id, error = %error, "follow-up failed");
        }
    });
}

async fn run_investigation(state: Arc<AppState>, investigation_id: &str) -> Result<()> {
    let investigation = db::get_investigation_row(&state.db, investigation_id).await?;
    let sections = db::get_sections(&state.db, investigation_id).await?;

    db::set_investigation_status(
        &state.db,
        investigation_id,
        "running",
        Some("Dang chay pipeline phan tich gon."),
    )
    .await?;
    db::upsert_heartbeat(
        &state.db,
        "investigation",
        investigation_id,
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "topic": investigation.topic }),
    )
    .await?;
    publish_investigation_update(state.clone(), investigation_id).await?;

    run_coordinator_step(state.clone(), &investigation, &sections).await?;
    short_pause().await;

    let sources = run_source_scout_step(state.clone(), &investigation, &sections).await?;
    short_pause().await;

    let analysis =
        run_technical_analyst_step(state.clone(), &investigation, &sections, &sources).await?;
    short_pause().await;

    run_evidence_verifier_step(
        state.clone(),
        &investigation,
        &sections,
        &analysis,
        &sources,
    )
    .await?;
    short_pause().await;

    let final_report = run_report_synthesizer_step(
        state.clone(),
        &investigation,
        &sections,
        &analysis,
        &sources,
    )
    .await?;

    let summary = format!(
        "Hoan thanh {} nguon, bias {}, confidence {}%.",
        sources.len(),
        analysis.bias,
        (analysis.confidence * 100.0).round() as i64
    );
    db::finish_investigation(&state.db, investigation_id, &summary, &final_report).await?;
    db::upsert_heartbeat(
        &state.db,
        "investigation",
        investigation_id,
        "completed",
        state.config.heartbeat.agent_ttl_seconds,
        json!({
            "sources": sources.len(),
            "bias": analysis.bias,
            "confidence": analysis.confidence,
        }),
    )
    .await?;

    publish_investigation_update(state.clone(), investigation_id).await?;
    state.events.publish(
        "run.completed",
        Some(investigation_id.to_string()),
        &json!({
            "investigation_id": investigation_id,
            "summary": summary,
            "final_report": final_report,
        }),
    );

    Ok(())
}

async fn run_coordinator_step(
    state: Arc<AppState>,
    investigation: &InvestigationRow,
    sections: &[SectionRow],
) -> Result<()> {
    let run_id = db::start_agent_run(&state.db, &investigation.id, AgentRole::Coordinator).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::Coordinator.as_str(),
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    let message = db::insert_message(
        &state.db,
        &investigation.id,
        sections.first().map(|section| section.id.as_str()),
        AgentRole::Coordinator,
        Some(AgentRole::SourceScout),
        "plan",
        &format!(
            "Topic: {}\nGoal: {}\nPipeline: Source Scout -> Technical Analyst -> Evidence Verifier -> Report Synthesizer.",
            investigation.topic, investigation.goal
        ),
        &[],
        Some(0.8),
    )
    .await?;
    publish_message(&state, &investigation.id, message);

    db::finish_agent_run(&state.db, &run_id).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::Coordinator.as_str(),
        "healthy",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    Ok(())
}

async fn run_source_scout_step(
    state: Arc<AppState>,
    investigation: &InvestigationRow,
    sections: &[SectionRow],
) -> Result<Vec<SourceDocumentRow>> {
    let run_id = db::start_agent_run(&state.db, &investigation.id, AgentRole::SourceScout).await?;
    let section = sections
        .iter()
        .find(|section| section.slug == "source_scope");
    if let Some(section) = section {
        db::set_section_status(&state.db, &section.id, "in_progress").await?;
    }
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::SourceScout.as_str(),
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    let source_specs = build_source_specs(investigation)
        .into_iter()
        .take(state.config.orchestration.max_parallel_sources.max(1))
        .collect::<Vec<_>>();

    let mut created = Vec::new();
    for spec in source_specs {
        let source = db::insert_source_document(
            &state.db,
            &investigation.id,
            &spec.url,
            &spec.title,
            Some(&spec.excerpt),
            spec.metadata,
        )
        .await?;

        let citation = build_citation(
            &source,
            "Seed public source prepared for this investigation.",
        );
        let message = db::insert_message(
            &state.db,
            &investigation.id,
            section.map(|item| item.id.as_str()),
            AgentRole::SourceScout,
            Some(AgentRole::TechnicalAnalyst),
            "evidence",
            &format!("Da dua nguon `{}` vao bo phan tich.", source.title),
            &[citation],
            Some(0.72),
        )
        .await?;
        publish_message(&state, &investigation.id, message);
        created.push(source);
        short_pause().await;
    }

    let conclusion = if created.is_empty() {
        "Khong co seed URL hop le, can bo sung nguon public de lam day transcript.".to_string()
    } else {
        format!("Da chuan hoa {} nguon public de phan tich.", created.len())
    };

    if let Some(section) = section {
        db::conclude_section(&state.db, &section.id, &conclusion).await?;
        state.events.publish(
            "section.concluded",
            Some(investigation.id.clone()),
            &json!({ "section_id": section.id, "conclusion": conclusion }),
        );
    }

    db::finish_agent_run(&state.db, &run_id).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::SourceScout.as_str(),
        if created.is_empty() {
            "warning"
        } else {
            "healthy"
        },
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id, "sources": created.len() }),
    )
    .await?;

    Ok(created)
}

async fn run_technical_analyst_step(
    state: Arc<AppState>,
    investigation: &InvestigationRow,
    sections: &[SectionRow],
    sources: &[SourceDocumentRow],
) -> Result<AnalysisSnapshot> {
    let run_id =
        db::start_agent_run(&state.db, &investigation.id, AgentRole::TechnicalAnalyst).await?;
    let section = sections
        .iter()
        .find(|section| section.slug == "technical_signals");
    if let Some(section) = section {
        db::set_section_status(&state.db, &section.id, "in_progress").await?;
    }
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::TechnicalAnalyst.as_str(),
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id, "sources": sources.len() }),
    )
    .await?;

    let snapshot = analyze_investigation(investigation, sources);
    let evidence = sources
        .iter()
        .take(2)
        .map(|source| {
            build_citation(
                source,
                "Public source referenced in the technical snapshot.",
            )
        })
        .collect::<Vec<_>>();

    let title = snapshot
        .pair_hint
        .as_ref()
        .map(|pair| format!("{} technical bias", pair))
        .unwrap_or_else(|| "Technical bias".to_string());
    let finding = db::insert_finding(
        &state.db,
        &investigation.id,
        section.map(|item| item.id.as_str()),
        AgentRole::TechnicalAnalyst,
        "trend",
        &title,
        &snapshot.summary,
        Some(&snapshot.bias),
        snapshot.confidence,
        &evidence,
    )
    .await?;
    publish_finding(&state, &investigation.id, finding);

    if !snapshot.levels.is_empty() {
        let level_finding = db::insert_finding(
            &state.db,
            &investigation.id,
            section.map(|item| item.id.as_str()),
            AgentRole::TechnicalAnalyst,
            "level",
            "Key levels to watch",
            &format!(
                "Cac muc dang duoc uu tien theo doi: {}.",
                snapshot.levels.join(", ")
            ),
            Some(&snapshot.bias),
            (snapshot.confidence - 0.05).clamp(0.2, 0.95),
            &evidence,
        )
        .await?;
        publish_finding(&state, &investigation.id, level_finding);
    }

    let message = db::insert_message(
        &state.db,
        &investigation.id,
        section.map(|item| item.id.as_str()),
        AgentRole::TechnicalAnalyst,
        Some(AgentRole::EvidenceVerifier),
        "evidence",
        &format!(
            "Bias hien tai: {}. Confidence: {}%. {}",
            snapshot.bias,
            (snapshot.confidence * 100.0).round() as i64,
            if snapshot.levels.is_empty() {
                "Chua co key level noi bat.".to_string()
            } else {
                format!("Key levels: {}.", snapshot.levels.join(", "))
            }
        ),
        &evidence,
        Some(snapshot.confidence),
    )
    .await?;
    publish_message(&state, &investigation.id, message);

    let conclusion = format!(
        "Da tong hop bias {} voi confidence {}%.",
        snapshot.bias,
        (snapshot.confidence * 100.0).round() as i64
    );
    if let Some(section) = section {
        db::conclude_section(&state.db, &section.id, &conclusion).await?;
        state.events.publish(
            "section.concluded",
            Some(investigation.id.clone()),
            &json!({ "section_id": section.id, "conclusion": conclusion }),
        );
    }

    db::finish_agent_run(&state.db, &run_id).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::TechnicalAnalyst.as_str(),
        "healthy",
        state.config.heartbeat.agent_ttl_seconds,
        json!({
            "investigation_id": investigation.id,
            "bias": snapshot.bias,
            "confidence": snapshot.confidence,
        }),
    )
    .await?;

    Ok(snapshot)
}

async fn run_evidence_verifier_step(
    state: Arc<AppState>,
    investigation: &InvestigationRow,
    sections: &[SectionRow],
    snapshot: &AnalysisSnapshot,
    sources: &[SourceDocumentRow],
) -> Result<()> {
    let run_id =
        db::start_agent_run(&state.db, &investigation.id, AgentRole::EvidenceVerifier).await?;
    let section = sections
        .iter()
        .find(|section| section.slug == "contradictions");
    if let Some(section) = section {
        db::set_section_status(&state.db, &section.id, "in_progress").await?;
    }
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::EvidenceVerifier.as_str(),
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    let critique = if snapshot.contradiction {
        "Narrative hien tai con mixed, can uu tien them xac nhan truoc khi dung mot huong duy nhat."
    } else {
        "Narrative hien tai kha dong huong, chua thay mau thuan lon giua cac dau moi duoc nap vao."
    };
    let evidence = sources
        .iter()
        .take(2)
        .map(|source| build_citation(source, "Source reused during contradiction check."))
        .collect::<Vec<_>>();
    let message = db::insert_message(
        &state.db,
        &investigation.id,
        section.map(|item| item.id.as_str()),
        AgentRole::EvidenceVerifier,
        Some(AgentRole::ReportSynthesizer),
        "critique",
        critique,
        &evidence,
        Some(if snapshot.contradiction { 0.62 } else { 0.74 }),
    )
    .await?;
    publish_message(&state, &investigation.id, message);

    if snapshot.contradiction {
        let finding = db::insert_finding(
            &state.db,
            &investigation.id,
            section.map(|item| item.id.as_str()),
            AgentRole::EvidenceVerifier,
            "contradiction",
            "Contradictory signals",
            critique,
            Some("mixed"),
            0.62,
            &evidence,
        )
        .await?;
        publish_finding(&state, &investigation.id, finding);
    }

    if let Some(section) = section {
        db::conclude_section(&state.db, &section.id, critique).await?;
        state.events.publish(
            "section.concluded",
            Some(investigation.id.clone()),
            &json!({ "section_id": section.id, "conclusion": critique }),
        );
    }

    db::finish_agent_run(&state.db, &run_id).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::EvidenceVerifier.as_str(),
        if snapshot.contradiction {
            "warning"
        } else {
            "healthy"
        },
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id, "contradiction": snapshot.contradiction }),
    )
    .await?;

    Ok(())
}

async fn run_report_synthesizer_step(
    state: Arc<AppState>,
    investigation: &InvestigationRow,
    sections: &[SectionRow],
    snapshot: &AnalysisSnapshot,
    sources: &[SourceDocumentRow],
) -> Result<String> {
    let run_id =
        db::start_agent_run(&state.db, &investigation.id, AgentRole::ReportSynthesizer).await?;
    let section = sections
        .iter()
        .find(|section| section.slug == "final_synthesis");
    if let Some(section) = section {
        db::set_section_status(&state.db, &section.id, "in_progress").await?;
    }
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::ReportSynthesizer.as_str(),
        "running",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    let final_report = build_final_report(investigation, snapshot, sources);
    let message = db::insert_message(
        &state.db,
        &investigation.id,
        section.map(|item| item.id.as_str()),
        AgentRole::ReportSynthesizer,
        Some(AgentRole::Coordinator),
        "synthesis",
        "Final report da san sang tren detail page.",
        &[],
        Some(snapshot.confidence),
    )
    .await?;
    publish_message(&state, &investigation.id, message);

    if let Some(section) = section {
        let conclusion = format!(
            "Bao cao cuoi da duoc dong goi, bias {} voi {} nguon.",
            snapshot.bias,
            sources.len()
        );
        db::conclude_section(&state.db, &section.id, &conclusion).await?;
        state.events.publish(
            "section.concluded",
            Some(investigation.id.clone()),
            &json!({ "section_id": section.id, "conclusion": conclusion }),
        );
    }

    db::finish_agent_run(&state.db, &run_id).await?;
    db::upsert_heartbeat(
        &state.db,
        "agent",
        AgentRole::ReportSynthesizer.as_str(),
        "healthy",
        state.config.heartbeat.agent_ttl_seconds,
        json!({ "investigation_id": investigation.id }),
    )
    .await?;

    Ok(final_report)
}

async fn answer_follow_up(
    state: Arc<AppState>,
    investigation_id: &str,
    question: &str,
    target_section_id: Option<&str>,
) -> Result<()> {
    let user_message = db::insert_message(
        &state.db,
        investigation_id,
        target_section_id,
        AgentRole::User,
        Some(AgentRole::Coordinator),
        "follow_up",
        question.trim(),
        &[],
        None,
    )
    .await?;
    publish_message(&state, investigation_id, user_message);

    let sections = db::get_sections(&state.db, investigation_id).await?;
    let findings = db::get_findings(&state.db, investigation_id).await?;
    let response = build_follow_up_answer(question, target_section_id, &sections, &findings);
    let answer = db::insert_message(
        &state.db,
        investigation_id,
        target_section_id,
        AgentRole::Coordinator,
        Some(AgentRole::User),
        "follow_up",
        &response,
        &[],
        Some(0.7),
    )
    .await?;
    publish_message(&state, investigation_id, answer);

    db::set_investigation_status(
        &state.db,
        investigation_id,
        "completed",
        Some("Da xu ly follow-up gan nhat."),
    )
    .await?;
    publish_investigation_update(state, investigation_id).await?;

    Ok(())
}

fn analyze_investigation(
    investigation: &InvestigationRow,
    sources: &[SourceDocumentRow],
) -> AnalysisSnapshot {
    let tags = db::from_json_vec::<String>(&investigation.tags_json).join(" ");
    let seed_urls = db::from_json_vec::<String>(&investigation.seed_urls_json).join(" ");
    let haystack = format!(
        "{} {} {} {}",
        investigation.topic, investigation.goal, tags, seed_urls
    );
    let pair_hint = extract_pair_hint(&haystack);
    let bias = detect_bias(&haystack);
    let contradiction = has_conflict_keywords(&haystack) || bias == "mixed";
    let levels = extract_levels(&haystack);

    let mut confidence = match bias.as_str() {
        "bullish" | "bearish" => 0.72,
        _ => 0.58,
    };
    confidence += (sources.len().min(3) as f64) * 0.05;
    if contradiction {
        confidence -= 0.08;
    }
    confidence = confidence.clamp(0.25, 0.92);

    let subject = pair_hint
        .clone()
        .unwrap_or_else(|| investigation.topic.clone());
    let summary = if levels.is_empty() {
        format!(
            "{} hien nghieng {} voi confidence {}%. Chua co key level noi bat duoc xac nhan tu brief hien tai.",
            subject,
            bias,
            (confidence * 100.0).round() as i64
        )
    } else {
        format!(
            "{} hien nghieng {} voi confidence {}%. Key levels uu tien theo doi: {}.",
            subject,
            bias,
            (confidence * 100.0).round() as i64,
            levels.join(", ")
        )
    };

    AnalysisSnapshot {
        pair_hint,
        bias,
        confidence,
        levels,
        contradiction,
        summary,
    }
}

fn build_final_report(
    investigation: &InvestigationRow,
    snapshot: &AnalysisSnapshot,
    sources: &[SourceDocumentRow],
) -> String {
    let source_list = if sources.is_empty() {
        "Khong co nguon nao duoc nap vao run nay.".to_string()
    } else {
        sources
            .iter()
            .take(3)
            .map(|source| format!("- {} ({})", source.title, source.url))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let level_line = if snapshot.levels.is_empty() {
        "Key levels: chua ro".to_string()
    } else {
        format!("Key levels: {}", snapshot.levels.join(", "))
    };

    format!(
        "Tom tat\n- Topic: {}\n- Bias: {}\n- Confidence: {}%\n- {}\n\nDanh gia\n{}\n\nNguon tham chieu\n{}\n\nGoi y tiep theo\n- Neu can follow-up, hay hoi ro hon ve timeframe, key levels hoac mau thuan giua cac narrative.",
        investigation.topic,
        snapshot.bias,
        (snapshot.confidence * 100.0).round() as i64,
        level_line,
        snapshot.summary,
        source_list,
    )
}

fn build_follow_up_answer(
    question: &str,
    target_section_id: Option<&str>,
    sections: &[SectionRow],
    findings: &[FindingRow],
) -> String {
    let target_section = target_section_id
        .and_then(|section_id| sections.iter().find(|section| section.id == section_id));

    let mut lines = vec![format!("Cau hoi: {}", question.trim())];
    if let Some(section) = target_section {
        lines.push(format!("Section muc tieu: {}", section.title));
        if let Some(conclusion) = &section.conclusion {
            lines.push(format!("Ket luan hien tai: {}", conclusion));
        }
    }

    for finding in findings.iter().take(3) {
        lines.push(format!(
            "- {} [{}]: {}",
            finding.title,
            finding
                .direction
                .clone()
                .unwrap_or_else(|| finding.kind.clone()),
            finding.summary
        ));
    }

    if findings.is_empty() {
        lines.push("Chua co finding nao de mo rong. Nen bo sung them seed URL cu the.".to_string());
    }

    lines.join("\n")
}

fn publish_message(state: &AppState, investigation_id: &str, row: crate::models::AgentMessageRow) {
    state.events.publish(
        "agent.message",
        Some(investigation_id.to_string()),
        &db::to_message_view(row),
    );
}

fn publish_finding(state: &AppState, investigation_id: &str, row: FindingRow) {
    state.events.publish(
        "finding.created",
        Some(investigation_id.to_string()),
        &db::to_finding_view(row),
    );
}

async fn publish_investigation_update(state: Arc<AppState>, investigation_id: &str) -> Result<()> {
    let detail = db::build_investigation_detail(&state.db, investigation_id).await?;
    state.events.publish(
        "investigation.updated",
        Some(investigation_id.to_string()),
        &detail.investigation,
    );
    Ok(())
}

fn build_citation(source: &SourceDocumentRow, snippet: &str) -> Citation {
    Citation {
        source_id: source.id.clone(),
        url: source.url.clone(),
        title: source.title.clone(),
        snippet: snippet.to_string(),
    }
}

struct SourceSpec {
    url: String,
    title: String,
    excerpt: String,
    metadata: serde_json::Value,
}

fn build_source_specs(investigation: &InvestigationRow) -> Vec<SourceSpec> {
    let seed_urls = db::from_json_vec::<String>(&investigation.seed_urls_json);
    let urls = if seed_urls.is_empty() {
        fallback_seed_urls(&investigation.topic)
    } else {
        seed_urls
    };

    urls.into_iter()
        .map(|url| SourceSpec {
            title: source_title(&url),
            excerpt: format!(
                "Nguon public duoc dua vao de tong hop topic `{}`.",
                investigation.topic
            ),
            metadata: json!({ "kind": "seed", "host": host_of(&url) }),
            url,
        })
        .collect()
}

fn fallback_seed_urls(topic: &str) -> Vec<String> {
    if let Some(pair) = extract_pair_hint(topic) {
        let compact = pair.replace('/', "").to_lowercase();
        let dashed = pair.replace('/', "-").to_lowercase();
        return vec![
            format!("https://www.fxstreet.com/rates-charts/{}", compact),
            format!("https://www.investing.com/currencies/{}", dashed),
            format!(
                "https://www.tradingview.com/symbols/{}/ideas/",
                compact.to_uppercase()
            ),
        ];
    }

    let slug = slug_for_url(topic);
    vec![
        format!("https://www.fxstreet.com/search?q={}", slug),
        format!("https://www.investing.com/search/?q={}", slug),
        format!("https://www.tradingview.com/ideas/search/{}/", slug),
    ]
}

fn source_title(url: &str) -> String {
    let host = host_of(url);
    let tail = url
        .split('/')
        .skip_while(|part| !part.contains('.'))
        .skip(1)
        .filter(|part| !part.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join("/");

    if tail.is_empty() {
        host
    } else {
        format!("{} / {}", host, tail)
    }
}

fn host_of(url: &str) -> String {
    url.split("//")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

fn slug_for_url(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn extract_pair_hint(text: &str) -> Option<String> {
    for raw in text.split_whitespace() {
        let token = raw
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '/')
            .to_ascii_uppercase();

        if token.len() == 7
            && token.as_bytes()[3] == b'/'
            && token
                .chars()
                .enumerate()
                .all(|(idx, ch)| idx == 3 || ch.is_ascii_alphabetic())
        {
            return Some(token);
        }

        if token.len() == 6 && token.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return Some(format!("{}/{}", &token[0..3], &token[3..6]));
        }
    }

    None
}

fn detect_bias(text: &str) -> String {
    const BULLISH: &[&str] = &[
        "bullish",
        "buy",
        "breakout",
        "higher high",
        "uptrend",
        "support",
    ];
    const BEARISH: &[&str] = &[
        "bearish",
        "sell",
        "breakdown",
        "lower low",
        "downtrend",
        "resistance",
    ];

    let lower = text.to_ascii_lowercase();
    let bullish_hits = BULLISH.iter().filter(|item| lower.contains(**item)).count();
    let bearish_hits = BEARISH.iter().filter(|item| lower.contains(**item)).count();

    if bullish_hits > bearish_hits {
        "bullish".to_string()
    } else if bearish_hits > bullish_hits {
        "bearish".to_string()
    } else {
        "mixed".to_string()
    }
}

fn has_conflict_keywords(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_bull = ["bullish", "buy", "breakout", "uptrend"]
        .iter()
        .any(|item| lower.contains(item));
    let has_bear = ["bearish", "sell", "breakdown", "downtrend"]
        .iter()
        .any(|item| lower.contains(item));
    has_bull && has_bear
}

fn extract_levels(text: &str) -> Vec<String> {
    let mut levels = Vec::new();

    for raw in text.split_whitespace() {
        let token = raw.trim_matches(|ch: char| !(ch.is_ascii_digit() || ch == '.' || ch == ','));
        if token.len() < 4 {
            continue;
        }
        if !token.chars().any(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let normalized = token.trim_matches('.').trim_matches(',').replace(',', ".");
        if normalized.chars().filter(|ch| *ch == '.').count() > 1 {
            continue;
        }
        if !levels.contains(&normalized) {
            levels.push(normalized);
        }
        if levels.len() == 4 {
            break;
        }
    }

    levels
}

async fn short_pause() {
    sleep(Duration::from_millis(120)).await;
}
