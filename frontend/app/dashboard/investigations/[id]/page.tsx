"use client";

import { startTransition, useEffectEvent } from "react";
import { useParams } from "next/navigation";
import { motion } from "motion/react";
import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { SlideIn } from "@/components/dashboard/motion-primitives";
import { StatusPill } from "@/components/dashboard/status-pill";
import { fetchInvestigation } from "@/lib/intelligence-api";
import {
  formatDateTime,
  formatRelativeTime,
  truncate,
} from "@/lib/formatting";
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

  const handleStreamEvent = useEffectEvent((_event: AppStreamEvent) => {
    startTransition(() => {
      reload();
    });
  });

  const streamStatus = useInvestigationStream(investigationId, handleStreamEvent);

  if (!investigationId) {
    return (
      <EmptyState
        title="Investigation khong hop le"
        description="Khong tim thay investigation id tren route hien tai."
      />
    );
  }

  if (loading && !data) {
    return (
      <EmptyState
        title="Dang tai investigation"
        description="Snapshot investigation dang duoc dong bo tu backend."
      />
    );
  }

  if (error && !data) {
    return <EmptyState title="Khong tai duoc investigation" description={error} />;
  }

  if (!data) {
    return (
      <EmptyState
        title="Khong co du lieu"
        description="Investigation nay chua duoc backend tra ve."
      />
    );
  }

  const summaryText =
    data.investigation.summary ||
    data.investigation.goal ||
    "Chua co ghi chu tom tat nao duoc luu cho investigation nay.";

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
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
              Created
            </div>
            <div className="mt-2 font-semibold">
              {formatDateTime(data.investigation.created_at)}
            </div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
              Updated
            </div>
            <div className="mt-2 font-semibold">
              {formatRelativeTime(data.investigation.updated_at)}
            </div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
              Seed URLs
            </div>
            <div className="mt-2 font-semibold">
              {data.investigation.seed_urls.length}
            </div>
          </div>
          <div className="border border-border bg-card p-4 text-[12px]">
            <div className="text-[10px] font-bold uppercase tracking-[1px] text-text-secondary">
              Sections
            </div>
            <div className="mt-2 font-semibold">{data.sections.length}</div>
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
              <h3 className="text-[14px] font-semibold">Section Status</h3>
            </div>
            <div className="grid grid-cols-2 gap-4 p-5">
              {data.sections.map((section) => (
                <div key={section.id} className="border border-border bg-card-alt p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-[12px] font-semibold">{section.title}</div>
                    <StatusPill value={section.status} />
                  </div>
                  <p className="mt-3 text-[11px] leading-relaxed text-text-secondary">
                    {section.conclusion || "Chua co conclusion duoc luu cho section nay."}
                  </p>
                </div>
              ))}
            </div>
          </div>

          <div className="border border-border bg-card">
            <div className="border-b border-border px-5 py-3">
              <h3 className="text-[14px] font-semibold">Stored Summary</h3>
            </div>
            <div className="space-y-4 px-5 py-4">
              <pre className="whitespace-pre-wrap text-[12px] leading-relaxed text-text-secondary">
                {summaryText}
              </pre>
              {data.investigation.tags.length ? (
                <div className="flex flex-wrap gap-2">
                  {data.investigation.tags.map((tag) => (
                    <span
                      key={tag}
                      className="border border-border bg-card-alt px-2 py-1 text-[10px] uppercase tracking-[0.8px] text-text-secondary"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              ) : null}
            </div>
          </div>
        </motion.div>
      </div>

      <SlideIn direction="right" delay={0.2}>
        <div className="-m-6 w-[360px] shrink-0 space-y-4 border-l border-border bg-panel p-5">
          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
              Seed URLs
            </h3>
            <div className="space-y-3">
              {data.investigation.seed_urls.length ? (
                data.investigation.seed_urls.map((url) => (
                  <a
                    key={url}
                    href={url}
                    target="_blank"
                    rel="noreferrer"
                    className="block border border-border bg-card px-3 py-3 transition-colors hover:border-cyan/30"
                  >
                    <div className="text-[11px] font-semibold">{truncate(url, 64)}</div>
                    <div className="mt-2 text-[10px] text-text-muted">Open source link</div>
                  </a>
                ))
              ) : (
                <EmptyState
                  title="Chua co seed URL"
                  description="Investigation nay chua duoc luu kem danh sach URL dau vao."
                />
              )}
            </div>
          </div>

          <div className="h-px bg-gradient-to-r from-transparent via-border to-transparent" />

          <div>
            <h3 className="mb-3 text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
              Heartbeats
            </h3>
            <div className="space-y-2">
              {data.heartbeats.length ? (
                data.heartbeats.map((heartbeat) => (
                  <div
                    key={`${heartbeat.component}-${heartbeat.scope}`}
                    className="flex items-center justify-between border border-border bg-card px-3 py-2"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-[11px] font-semibold">
                        {heartbeat.component}/{heartbeat.scope}
                      </div>
                      <div className="text-[10px] text-text-muted">
                        {formatRelativeTime(heartbeat.last_seen_at)}
                      </div>
                    </div>
                    <StatusPill value={heartbeat.health} />
                  </div>
                ))
              ) : (
                <EmptyState
                  title="Chua co heartbeat"
                  description="Heartbeat cua service, scheduler va investigation se xuat hien tai day."
                />
              )}
            </div>
          </div>
        </div>
      </SlideIn>
    </div>
  );
}
