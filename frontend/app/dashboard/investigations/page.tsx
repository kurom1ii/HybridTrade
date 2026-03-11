"use client";

import Link from "next/link";
import { Suspense, startTransition, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { motion } from "motion/react";
import { EmptyState } from "@/components/dashboard/empty-state";
import { PageTitle } from "@/components/dashboard/page-title";
import { StatusPill } from "@/components/dashboard/status-pill";
import { SlideIn } from "@/components/dashboard/motion-primitives";
import { createInvestigation, fetchInvestigations } from "@/lib/intelligence-api";
import { formatDateTime, formatRelativeTime, truncate } from "@/lib/formatting";
import { usePollingResource } from "@/hooks/use-polling-resource";

function readPrefillValue(value: string | null) {
  return (value ?? "").replace(/\r/g, "");
}

function InvestigationsPageContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { data, loading, error, reload } = usePollingResource("investigations", fetchInvestigations, {
    intervalMs: 10_000,
  });
  const [topic, setTopic] = useState("");
  const [goal, setGoal] = useState("");
  const [seedUrls, setSeedUrls] = useState("");
  const [tags, setTags] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const prefillKey = searchParams.toString();
  const prefilledPair = searchParams.get("pair") ?? "";

  useEffect(() => {
    const pair = searchParams.get("pair") ?? "";
    const nextTopic =
      readPrefillValue(searchParams.get("topic")) ||
      (pair ? `${pair} forex technical intelligence` : "");
    const nextGoal = readPrefillValue(searchParams.get("goal"));
    const nextTags = readPrefillValue(searchParams.get("tags"));
    const nextSeedUrls = readPrefillValue(searchParams.get("seed_urls"));

    setTopic(nextTopic);
    setGoal(nextGoal);
    setTags(nextTags);
    setSeedUrls(nextSeedUrls);
    setSubmitError(null);
  }, [prefillKey]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!topic.trim()) {
      setSubmitError("Topic la bat buoc.");
      return;
    }

    setSubmitting(true);
    setSubmitError(null);
    try {
      const detail = await createInvestigation({
        topic: topic.trim(),
        goal: goal.trim() || undefined,
        tags: tags
          .split(",")
          .map((value) => value.trim())
          .filter(Boolean),
        seed_urls: seedUrls
          .split(/\n|,/)
          .map((value) => value.trim())
          .filter(Boolean),
        source_scope: "public_web",
        priority: prefilledPair ? "high" : undefined,
      });
      startTransition(() => {
        router.push(`/dashboard/investigations/${detail.investigation.id}`);
      });
    } catch (creationError) {
      setSubmitError(
        creationError instanceof Error ? creationError.message : "Khong tao duoc investigation",
      );
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex h-full flex-col gap-6 xl:flex-row overflow-y-auto p-6">
      <div className="min-w-0 flex-1">
        <div className="flex flex-col gap-6">
          <PageTitle
            title="Investigation Composer"
            subtitle="Tao run moi cho forex pair, strategy brief hoac thematic scan roi theo doi transcript va ket luan tren detail page."
            breadcrumb="DASHBOARD / INVESTIGATIONS"
          />

          {prefilledPair ? (
            <div className="border border-cyan/30 bg-cyan-dim px-4 py-3 text-[12px] text-cyan">
              Composer da duoc prefill cho <span className="font-semibold">{prefilledPair}</span>.
              Ban co the sua lai topic, tags va seed URLs truoc khi spawn subagents.
            </div>
          ) : null}

          <motion.form
            onSubmit={handleSubmit}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4 }}
            className="space-y-4 border border-border bg-card p-5"
          >
            <div className="grid gap-4 xl:grid-cols-2">
              <label className="space-y-2">
                <span className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
                  Topic
                </span>
                <input
                  value={topic}
                  onChange={(event) => setTopic(event.target.value)}
                  placeholder="Vi du: EUR/USD daily technical scan from public forex commentary"
                  className="h-11 w-full border border-border bg-input px-3 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
                />
              </label>
              <label className="space-y-2">
                <span className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
                  Tags
                </span>
                <input
                  value={tags}
                  onChange={(event) => setTags(event.target.value)}
                  placeholder="eurusd, forex, momentum, breakout"
                  className="h-11 w-full border border-border bg-input px-3 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
                />
              </label>
            </div>

            <label className="block space-y-2">
              <span className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
                Goal
              </span>
              <textarea
                value={goal}
                onChange={(event) => setGoal(event.target.value)}
                rows={4}
                placeholder="Muc tieu: tong hop technical bias, key levels, timeframe uu tien, confidence va cac narrative mau thuan tren cac nguon public web."
                className="w-full border border-border bg-input px-3 py-3 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
              />
            </label>

            <label className="block space-y-2">
              <span className="text-[11px] font-bold uppercase tracking-[1px] text-text-secondary">
                Seed URLs
              </span>
              <textarea
                value={seedUrls}
                onChange={(event) => setSeedUrls(event.target.value)}
                rows={5}
                placeholder="Moi dong mot URL public web, vi du FXStreet / Investing / TradingView ideas cho cap tien dang nghien cuu."
                className="w-full border border-border bg-input px-3 py-3 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
              />
            </label>

            {submitError ? <div className="text-[12px] text-loss">{submitError}</div> : null}

            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="text-[11px] text-text-secondary">
                Scheduler, memory, SQLite va heartbeat se cap nhat tu dong sau khi run bat dau.
              </div>
              <button
                type="submit"
                disabled={submitting}
                className="bg-cyan px-4 py-2 text-[11px] font-bold tracking-[1px] text-black transition-colors hover:bg-cyan/90 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {submitting ? "SUBMITTING..." : prefilledPair ? "SPAWN FOREX SUBAGENTS" : "START INVESTIGATION"}
              </button>
            </div>
          </motion.form>

          <div className="border border-border bg-card">
            <div className="flex items-center justify-between border-b border-border px-5 py-3">
              <div>
                <h3 className="text-[14px] font-semibold">Investigation Queue</h3>
                <p className="mt-1 text-[11px] text-text-secondary">
                  Queue hien thi cac run da tao, bao gom pair-specific scans va broader research briefs.
                </p>
              </div>

              <button
                onClick={reload}
                className="px-3 py-1.5 text-[11px] font-bold tracking-[0.8px] text-text-secondary transition-colors hover:bg-secondary hover:text-foreground"
              >
                REFRESH
              </button>
            </div>

            <div className="divide-y divide-border">
              {loading && !data ? (
                <div className="px-5 py-6 text-[12px] text-text-secondary">Dang tai queue...</div>
              ) : null}
              {error && !data ? <div className="px-5 py-6 text-[12px] text-loss">{error}</div> : null}
              {!loading && data && data.length === 0 ? (
                <div className="px-5 py-6">
                  <EmptyState
                    title="Queue dang rong"
                    description="Tao investigation dau tien o form ben tren de khoi dong doi multi-agent."
                  />
                </div>
              ) : null}
              {data?.map((item) => (
                <Link
                  key={item.id}
                  href={`/dashboard/investigations/${item.id}`}
                  className="block px-5 py-4 transition-colors hover:bg-card-alt"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                      <div className="flex items-center gap-3">
                        <h4 className="truncate text-[14px] font-semibold">{item.topic}</h4>
                        <StatusPill value={item.status} />
                      </div>
                      <p className="mt-2 text-[12px] leading-relaxed text-text-secondary">
                        {truncate(item.summary || item.goal, 180)}
                      </p>
                      <div className="mt-3 flex flex-wrap gap-3 text-[10px] uppercase tracking-[0.8px] text-text-muted">
                        <span>{item.priority}</span>
                        <span>{item.source_scope}</span>
                        <span>{formatRelativeTime(item.updated_at)}</span>
                      </div>
                    </div>
                    <div className="shrink-0 text-right text-[10px] text-text-muted">
                      <div>{formatDateTime(item.created_at)}</div>
                      <div className="mt-1">{item.seed_urls.length} URLs</div>
                    </div>
                  </div>
                </Link>
              ))}
            </div>
          </div>
        </div>
      </div>

      <SlideIn direction="right" delay={0.2}>
        <div className="w-full shrink-0 xl:-m-6 xl:w-[320px] xl:border-l xl:border-border xl:bg-panel xl:p-5">
          <div className="flex flex-col gap-4">
            <div className="border border-border bg-card p-4">
              <h3 className="text-[12px] font-semibold">Composer Guidance</h3>
              <div className="mt-3 flex flex-col gap-3 text-[12px] leading-relaxed text-text-secondary">
                <p>1. Neu la forex pair, nen neu ro symbol va timeframe trong topic hoac goal.</p>
                <p>2. Seed URLs nen co 2-5 nguon commentary public de tang chat luong evidence.</p>
                <p>3. Neu can tuy bien nhanh, vao dashboard chon pair roi bam Customize brief.</p>
              </div>
            </div>

            <div className="border border-border bg-card p-4">
              <h3 className="text-[12px] font-semibold">Quick Links</h3>
              <div className="mt-3 flex flex-col gap-3 text-[11px] font-bold uppercase tracking-[0.8px]">
                <Link href="/dashboard" className="text-cyan transition-colors hover:text-foreground">
                  Ve forex command center
                </Link>
                <Link href="/dashboard/agents" className="text-text-secondary transition-colors hover:text-foreground">
                  Xem agent status
                </Link>
                <Link href="/dashboard/analytics" className="text-text-secondary transition-colors hover:text-foreground">
                  Xem schedules / cron jobs
                </Link>
              </div>
            </div>
          </div>
        </div>
      </SlideIn>
    </div>
  );
}

export default function InvestigationsPage() {
  return (
    <Suspense
      fallback={
        <EmptyState
          title="Dang mo investigation composer"
          description="Dang dong bo prefill parameters cho form tao investigation."
        />
      }
    >
      <InvestigationsPageContent />
    </Suspense>
  );
}
