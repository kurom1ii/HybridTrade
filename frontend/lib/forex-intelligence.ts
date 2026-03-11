import {
  DashboardResponse,
  FindingView,
  InvestigationDetail,
  InvestigationSummary,
  MessageView,
} from "@/lib/intelligence-types";
import {
  FOREX_CATEGORY_ORDER,
  FOREX_PAIRS,
  ForexCategory,
  ForexPair,
  recommendedSourceUrls,
} from "@/lib/forex-pairs";

export type PairCoverageStatus = "covered" | "running" | "queued" | "uncovered";
export type PairBias = "bullish" | "bearish" | "mixed" | "awaiting";

export interface ForexPairInsight {
  pair: ForexPair;
  coverageStatus: PairCoverageStatus;
  bias: PairBias;
  confidence: number | null;
  evidenceCount: number;
  sourceCount: number;
  linkedInvestigation: InvestigationSummary | null;
  summary: string;
  keyLevels: string[];
  signals: string[];
  timeframe: string;
}

export interface ForexBoardData {
  totalPairs: number;
  coveredPairs: number;
  runningPairs: number;
  uncoveredPairs: number;
  avgConfidence: number | null;
  highConvictionPairs: number;
  pairs: ForexPairInsight[];
}

export interface SubagentTask {
  role: string;
  status: "completed" | "running" | "queued" | "awaiting";
  task: string;
  note: string;
}

const SIGNAL_KEYWORDS = [
  "rsi",
  "macd",
  "ema",
  "sma",
  "trendline",
  "support",
  "resistance",
  "breakout",
  "breakdown",
  "divergence",
  "volume",
  "momentum",
];

export function buildForexBoardData(
  dashboard: DashboardResponse,
  investigations: InvestigationSummary[],
): ForexBoardData {
  const pairs = FOREX_PAIRS.map((pair) => buildPairInsight(pair, dashboard, investigations));
  const coveredPairs = pairs.filter((pair) => pair.coverageStatus === "covered").length;
  const runningPairs = pairs.filter((pair) => pair.coverageStatus === "running" || pair.coverageStatus === "queued").length;
  const uncoveredPairs = pairs.filter((pair) => pair.coverageStatus === "uncovered").length;
  const confidences = pairs
    .map((pair) => pair.confidence)
    .filter((value): value is number => typeof value === "number");

  return {
    totalPairs: pairs.length,
    coveredPairs,
    runningPairs,
    uncoveredPairs,
    avgConfidence: confidences.length
      ? confidences.reduce((sum, value) => sum + value, 0) / confidences.length
      : null,
    highConvictionPairs: pairs.filter((pair) => (pair.confidence ?? 0) >= 0.75).length,
    pairs,
  };
}

export function categoryBreakdown(pairs: ForexPairInsight[]) {
  return FOREX_CATEGORY_ORDER.map((category) => {
    const group = pairs.filter((pair) => pair.pair.category === category);
    return {
      category,
      total: group.length,
      covered: group.filter((pair) => pair.coverageStatus === "covered").length,
      running: group.filter((pair) => pair.coverageStatus === "running" || pair.coverageStatus === "queued").length,
      pairs: group,
    };
  });
}

export function buildSubagentTasks(
  insight: ForexPairInsight,
  detail: InvestigationDetail | null,
): SubagentTask[] {
  const latestMessage = (role: string): MessageView | undefined =>
    detail?.transcript
      .filter((message) => message.agent_role === role)
      .slice()
      .reverse()[0];
  const sectionStatus = (slug: string) => detail?.sections.find((section) => section.slug === slug)?.status;

  return [
    {
      role: "Coordinator",
      status: detail
        ? detail.investigation.status === "completed"
          ? "completed"
          : detail.investigation.status === "running"
            ? "running"
            : "queued"
        : insight.coverageStatus === "uncovered"
          ? "awaiting"
          : insight.coverageStatus === "covered"
            ? "completed"
            : "queued",
      task: `Dieu phoi luong phan tich cho ${insight.pair.symbol}`,
      note:
        latestMessage("coordinator")?.content ||
        `Chia task cho cac agent con de tong hop narrative ky thuat cua ${insight.pair.symbol}.`,
    },
    {
      role: "Source Scout",
      status: detail
        ? detail.sources.length > 0
          ? "completed"
          : detail.investigation.status === "running"
            ? "running"
            : "queued"
        : insight.coverageStatus === "uncovered"
          ? "awaiting"
          : "queued",
      task: "Quet nguon cong khai va market commentary theo cap tien",
      note:
        latestMessage("source_scout")?.content ||
        `Nguon goi y: ${recommendedSourceUrls(insight.pair.symbol).join(" | ")}`,
    },
    {
      role: "Technical Analyst",
      status: detail
        ? detail.findings.length > 0 || sectionStatus("technical_signals") === "concluded"
          ? "completed"
          : sectionStatus("technical_signals") === "in_progress"
            ? "running"
            : "queued"
        : insight.coverageStatus === "covered"
          ? "completed"
          : insight.coverageStatus === "running"
            ? "running"
            : "awaiting",
      task: "Rut ra bias, key levels, breakout va momentum signals",
      note:
        latestMessage("technical_analyst")?.content ||
        insight.summary,
    },
    {
      role: "Evidence Verifier",
      status: detail
        ? sectionStatus("contradictions") === "concluded"
          ? "completed"
          : sectionStatus("contradictions") === "in_progress"
            ? "running"
            : "queued"
        : insight.coverageStatus === "covered"
          ? "completed"
          : "awaiting",
      task: "Doi chieu mau thuan giua cac nguon va xep hang confidence",
      note:
        latestMessage("evidence_verifier")?.content ||
        "Kiem tra xem narrative bullish/bearish co xung dot hay khong.",
    },
    {
      role: "Report Synthesizer",
      status: detail
        ? detail.investigation.final_report
          ? "completed"
          : detail.investigation.status === "running"
            ? "running"
            : "queued"
        : insight.coverageStatus === "covered"
          ? "completed"
          : "awaiting",
      task: "Tong hop phan tich ky thuat thanh de xuat doc duoc tren dashboard",
      note:
        latestMessage("report_synthesizer")?.content ||
        "Xuat final report voi muc confidence va cac diem can theo doi.",
    },
  ];
}

export function pairComposerDefaults(pair: ForexPair) {
  return {
    topic: `${pair.symbol} forex technical intelligence`,
    goal: `Tong hop phan tich ky thuat cho cap ${pair.symbol}, bao gom xu huong, key levels, confidence va cac narrative mau thuan tu nguon public web.`,
    tags: [pair.symbol.toLowerCase(), pair.base.toLowerCase(), pair.quote.toLowerCase(), "forex"],
    seedUrls: recommendedSourceUrls(pair.symbol),
  };
}

function buildPairInsight(
  pair: ForexPair,
  dashboard: DashboardResponse,
  investigations: InvestigationSummary[],
): ForexPairInsight {
  const linkedInvestigation = investigations
    .filter((investigation) => investigationMatchesPair(investigation, pair))
    .sort((left, right) => Date.parse(right.updated_at) - Date.parse(left.updated_at))[0] ?? null;
  const relatedFindings = dashboard.recent_findings.filter((finding) => findingMatchesPair(finding, pair));
  const confidence = relatedFindings.length
    ? relatedFindings.reduce((sum, finding) => sum + finding.confidence, 0) / relatedFindings.length
    : null;
  const bias = deriveBias(relatedFindings, linkedInvestigation);
  const coverageStatus = deriveCoverageStatus(linkedInvestigation, relatedFindings.length > 0);
  const keyLevels = extractLevels(
    [
      ...relatedFindings.map((finding) => finding.summary),
      linkedInvestigation?.summary ?? "",
      linkedInvestigation?.final_report ?? "",
    ].join("\n"),
  );
  const signals = extractSignals(
    [
      ...relatedFindings.map((finding) => `${finding.title} ${finding.summary}`),
      linkedInvestigation?.goal ?? "",
      linkedInvestigation?.final_report ?? "",
    ].join("\n"),
  );

  return {
    pair,
    coverageStatus,
    bias,
    confidence,
    evidenceCount: relatedFindings.length,
    sourceCount: relatedFindings.reduce((count, finding) => count + finding.evidence.length, 0),
    linkedInvestigation,
    summary:
      relatedFindings[0]?.summary ||
      linkedInvestigation?.summary ||
      linkedInvestigation?.goal ||
      `Chua co du lieu AI cho ${pair.symbol}. Nen spawn subagents de thu thap source va tong hop technical bias.`,
    keyLevels,
    signals,
    timeframe: extractTimeframe(linkedInvestigation?.goal || linkedInvestigation?.summary || relatedFindings[0]?.summary || pair.note),
  };
}

function investigationMatchesPair(investigation: InvestigationSummary, pair: ForexPair) {
  const haystack = [
    investigation.topic,
    investigation.goal,
    investigation.summary || "",
    investigation.final_report || "",
    investigation.tags.join(" "),
  ].join("\n");

  return textMentionsPair(haystack, pair);
}

function findingMatchesPair(finding: FindingView, pair: ForexPair) {
  return textMentionsPair(`${finding.title}\n${finding.summary}`, pair);
}

function textMentionsPair(text: string, pair: ForexPair) {
  const upper = text.toUpperCase();
  const compact = pair.symbol.replace("/", "");
  return (
    upper.includes(pair.symbol) ||
    upper.includes(compact) ||
    upper.includes(`${pair.base}${pair.quote}`) ||
    upper.includes(`${pair.base}-${pair.quote}`)
  );
}

function deriveCoverageStatus(
  investigation: InvestigationSummary | null,
  hasFinding: boolean,
): PairCoverageStatus {
  if (!investigation && !hasFinding) return "uncovered";
  if (investigation?.status === "running") return "running";
  if (investigation?.status === "queued") return "queued";
  if (investigation?.status === "completed" || hasFinding) return "covered";
  return "uncovered";
}

function deriveBias(findings: FindingView[], investigation: InvestigationSummary | null): PairBias {
  if (findings.length > 0) {
    const bullish = findings.filter((finding) => finding.direction === "bullish").length;
    const bearish = findings.filter((finding) => finding.direction === "bearish").length;
    if (bullish > bearish) return "bullish";
    if (bearish > bullish) return "bearish";
    return "mixed";
  }

  const haystack = `${investigation?.summary || ""}\n${investigation?.final_report || ""}`.toLowerCase();
  if (haystack.includes("bullish")) return "bullish";
  if (haystack.includes("bearish")) return "bearish";
  if (haystack.includes("mixed") || haystack.includes("contradiction")) return "mixed";
  return "awaiting";
}

function extractLevels(text: string) {
  const matches = text.match(/\b\d{1,3}(?:\.\d{2,5})?\b/g) ?? [];
  const unique = Array.from(new Set(matches));
  return unique.slice(0, 4);
}

function extractSignals(text: string) {
  const lower = text.toLowerCase();
  return SIGNAL_KEYWORDS.filter((signal) => lower.includes(signal)).slice(0, 5);
}

function extractTimeframe(text: string) {
  const lower = text.toLowerCase();
  if (lower.includes("weekly")) return "Weekly";
  if (lower.includes("daily")) return "Daily";
  if (lower.includes("4h") || lower.includes("4 h")) return "4H";
  if (lower.includes("1h") || lower.includes("1 h")) return "1H";
  return "H4 / D1";
}

export function defaultSelectedPair(pairs: ForexPairInsight[]) {
  return pairs.find((pair) => pair.coverageStatus === "covered")
    || pairs.find((pair) => pair.coverageStatus === "running")
    || pairs[0]
    || null;
}

export function categoryMatches(category: ForexCategory | "All", insight: ForexPairInsight) {
  return category === "All" || insight.pair.category === category;
}

