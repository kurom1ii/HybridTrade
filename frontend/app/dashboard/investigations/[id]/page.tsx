"use client";

import Link from "next/link";
import { startTransition, useState, useEffectEvent } from "react";
import { useParams } from "next/navigation";
import { motion } from "motion/react";
import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { SlideIn } from "@/components/dashboard/motion-primitives";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchInvestigation, submitFollowUp } from "@/lib/intelligence-api";
import { formatDateTime, formatRelativeTime, titleFromRole, truncate } from "@/lib/formatting";
import { AppStreamEvent } from "@/lib/intelligence-types";
import { useInvestigationStream } from "@/hooks/use-investigation-stream";
import { usePollingResource } from "@/hooks/use-polling-resource";

export default function InvestigationDetailPage() {
  const params = useParams<{ id: string }>();
  const investigationId = typeof params.id === "string" ? params.id : null;
  const { data, loading, error, reload } = usePollingResource(
    `investigation-${investigationId ?? "missing"}`,
    () => fetchInvestigation(investigationId ?? ""),
    {
      enabled: Boolean(investigationId),
      intervalMs: 0,
    },
  );
  const [question, setQuestion] = useState("");
  const [targetSection, setTargetSection] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const handleStreamEvent = useEffectEvent((_event: AppStreamEvent) => {
    startTransition(() => {
      reload();
    });
  });

  const streamStatus = useInvestigationStream(investigationId, handleStreamEvent);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!investigationId || !question.trim()) return;
    setSubmitting(true);
    setSubmitError(null);
    try {
      await submitFollowUp(investigationId, {
        question: question.trim(),
        target_section: targetSection || undefined,
        reuse_sources: true,
      });
      setQuestion("");
    } catch (followUpError) {
      setSubmitError(followUpError instanceof Error ? followUpError.message : "Khong gui duoc follow-up");
    } finally {
      setSubmitting(false);
    }
  }

  if (!investigationId) {
    return <EmptyState title="Investigation khong hop le" description="Khong tim thay investigation id tren route hien tai." />;
  }

  if (loading && !data) {
    return <EmptyState title="Dang tai investigation" description="Snapshot, findings va transcript dang duoc dong bo tu backend." />;
  }

  if (error && !data) {
    return <EmptyState title="Khong tai duoc investigation" description={error} />;
  }

  if (!data) {
    return <EmptyState title="Khong co du lieu" description="Investigation nay chua duoc backend tra ve." />;
  }

  return (
    <div className="flex h-full gap-6 overflow-y-auto p-6">
      <div className="min-w-0 flex-1 space-y-6">
        <div className="flex items-start justify-between gap-4">
          <PageTitle
            title={data.investigation.topic}
            subtitle={data.investigation.goal}
            breadcrumb="DASHBOARD / INVESTIGATIONS / DETAIL"
          />
          <div className="space-y-2 text-right">
            <StatusPill value={data.investigation.status} />
            <div className="text-[10px] uppercase tracking-[0.8px] text-text-muted">
              Stream <StatusPill value={streamStatus} className="ml-2" />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-4 gap-4">
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">Created</div>
            <div className="mt-2 font-semibold">{formatDateTime(data.investigation.created_at)}</div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">Updated</div>
            <div className="mt-2 font-semibold">{formatRelativeTime(data.investigation.updated_at)}</div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">Seed URLs</div>
            <div className="mt-2 font-semibold">{data.investigation.seed_urls.length}</div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">Findings</div>
            <div className="mt-2 font-semibold">{data.findings.length}</div>
          </div>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
          className="space-y-4"
        >
          <div className="border border-border bg-card">
            <div className="border-b border-border px-5 py-3">
              <h3 className="text-[14px] font-semibold">Section Conclusions</h3>
            </div>
            <div className="grid grid-cols-2 gap-4 p-5">
              {data.sections.map((section) => (
                <div key={section.id} className="border border-border bg-card-alt p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-[12px] font-semibold">{section.title}</div>
                    <StatusPill value={section.status} />
                  </div>
                  <p className="mt-3 text-[11px] leading-relaxed text-text-secondary">
                    {section.conclusion || "Section dang duoc coordinator cap nhat."}
                  </p>
                </div>
              ))}
            </div>
          </div>

          <div className="border border-border bg-card">
            <div className="border-b border-border px-5 py-3">
              <h3 className="text-[14px] font-semibold">Final Report</h3>
            </div>
            <div className="px-5 py-4">
              <pre className="whitespace-pre-wrap text-[12px] leading-relaxed text-text-secondary">
                {data.investigation.final_report || "Bao cao tong hop se xuat hien sau khi Report Synthesizer hoan thanh."}
              </pre>
            </div>
          </div>

          <div className="border border-border bg-card">
            <div className="border-b border-border px-5 py-3">
              <h3 className="text-[14px] font-semibold">Agent Transcript</h3>
            </div>
            <div className="max-h-[520px] space-y-3 overflow-y-auto px-5 py-4">
              {data.transcript.length ? (
                data.transcript.map((message) => (
                  <div key={message.id} className="border border-border bg-card-alt p-4">
                    <div className="flex items-center justify-between gap-4 text-[10px] uppercase tracking-[0.8px] text-text-muted">
                      <div className="flex items-center gap-3">
                        <span className="font-bold text-cyan">{titleFromRole(message.agent_role)}</span>
                        <span>{message.kind}</span>
                        {message.target_role ? <span>→ {titleFromRole(message.target_role)}</span> : null}
                      </div>
                      <span>{formatRelativeTime(message.created_at)}</span>
                    </div>
                    <p className="mt-3 whitespace-pre-wrap text-[12px] leading-relaxed text-text-secondary">
                      {message.content}
                    </p>
                    {message.citations.length ? (
                      <div className="mt-3 space-y-2 border-t border-border pt-3">
                        {message.citations.map((citation) => (
                          <a
                            key={`${message.id}-${citation.source_id}`}
                            href={citation.url}
                            target="_blank"
                            rel="noreferrer"
                            className="block text-[11px] text-text-secondary transition-colors hover:text-cyan"
                          >
                            <span className="font-semibold">{citation.title}</span>
                            <span className="ml-2 text-text-muted">{truncate(citation.snippet, 120)}</span>
                          </a>
                        ))}
                      </div>
                    ) : null}
                  </div>
                ))
              ) : (
                <EmptyState
                  title="Transcript rong"
                  description="Coordinator va team members se xuat hien tai day khi investigation bat dau chay."
                />
              )}
            </div>
          </div>
        </motion.div>
      </div>

      <SlideIn direction="right" delay={0.2}>
        <div className="-m-6 w-[360px] shrink-0 space-y-4 border-l border-border bg-panel p-5">
          <form onSubmit={handleSubmit} className="border border-border bg-card p-4">
            <h3 className="text-[12px] font-semibold">Ask Follow-up</h3>
            <label className="mt-3 block space-y-2">
              <span className="text-[10px] font-bold uppercase tracking-[0.8px] text-text-secondary">Target section</span>
              <select
                value={targetSection}
                onChange={(event) => setTargetSection(event.target.value)}
                className="h-10 w-full border border-border bg-input px-3 text-[12px] focus:outline-none focus:ring-1 focus:ring-cyan"
              >
                <option value="">Tat ca sections</option>
                {data.sections.map((section) => (
                  <option key={section.id} value={section.id}>
                    {section.title}
                  </option>
                ))}
              </select>
            </label>

            <label className="mt-3 block space-y-2">
              <span className="text-[10px] font-bold uppercase tracking-[0.8px] text-text-secondary">Question</span>
              <textarea
                value={question}
                onChange={(event) => setQuestion(event.target.value)}
                rows={4}
                placeholder="Vi du: Agent hay lam ro hon mau thuan giua bullish va bearish narrative?"
                className="w-full border border-border bg-input px-3 py-3 text-[12px] focus:outline-none focus:ring-1 focus:ring-cyan"
              />
            </label>

            {submitError ? <div className="mt-3 text-[11px] text-loss">{submitError}</div> : null}

            <button
              type="submit"
              disabled={submitting}
              className="mt-4 w-full bg-cyan px-4 py-2 text-[11px] font-bold tracking-[1px] text-black transition-colors hover:bg-cyan/90 disabled:cursor-not-allowed disabled:opacity-60"
            >
              {submitting ? "SENDING..." : "SEND FOLLOW-UP"}
            </button>
          </form>

          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">Findings</h3>
            <div className="space-y-3">
              {data.findings.length ? (
                data.findings.map((finding) => (
                  <div key={finding.id} className="border border-border bg-card px-3 py-3">
                    <div className="flex items-center justify-between gap-3">
                      <div className="text-[11px] font-semibold text-cyan">{finding.title}</div>
                      <StatusPill value={finding.direction || finding.kind} className="shrink-0" />
                    </div>
                    <p className="mt-2 text-[11px] leading-relaxed text-text-secondary">{truncate(finding.summary, 150)}</p>
                    <div className="mt-2 text-[10px] text-text-muted">
                      {(finding.confidence * 100).toFixed(0)}% · {formatRelativeTime(finding.created_at)}
                    </div>
                  </div>
                ))
              ) : (
                <EmptyState
                  title="Chua co findings"
                  description="Findings tu Technical Analyst va Evidence Verifier se duoc liet ke tai day."
                />
              )}
            </div>
          </div>

          <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">Sources</h3>
            <div className="space-y-3">
              {data.sources.length ? (
                data.sources.map((source) => (
                  <Link
                    key={source.id}
                    href={source.url}
                    target="_blank"
                    className="block border border-border bg-card px-3 py-3 transition-colors hover:border-cyan/30"
                  >
                    <div className="text-[11px] font-semibold">{truncate(source.title, 80)}</div>
                    <p className="mt-1 text-[11px] leading-relaxed text-text-secondary">
                      {truncate(source.excerpt || source.url, 130)}
                    </p>
                    <div className="mt-2 text-[10px] text-text-muted">{formatDateTime(source.fetched_at)}</div>
                  </Link>
                ))
              ) : (
                <EmptyState
                  title="Chua thu thap nguon"
                  description="Source Scout se dua danh sach bai viet/website cong khai vao panel nay."
                />
              )}
            </div>
          </div>

          <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">Heartbeats</h3>
            <div className="space-y-2">
              {data.heartbeats.length ? (
                data.heartbeats.map((heartbeat) => (
                  <div key={`${heartbeat.component}-${heartbeat.scope}`} className="flex items-center justify-between border border-border bg-card px-3 py-2">
                    <div className="min-w-0">
                      <div className="truncate text-[11px] font-semibold">{heartbeat.component}/{heartbeat.scope}</div>
                      <div className="text-[10px] text-text-muted">{formatRelativeTime(heartbeat.last_seen_at)}</div>
                    </div>
                    <StatusPill value={heartbeat.health} />
                  </div>
                ))
              ) : (
                <EmptyState
                  title="Chua co heartbeat"
                  description="Heartbeat cua investigation, service, agent va tools se xuat hien tai day."
                />
              )}
            </div>
          </div>
        </div>
      </SlideIn>
    </div>
  );
}

