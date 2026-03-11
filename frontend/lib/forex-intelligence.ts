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
import { titleFromRole } from "@/lib/formatting";

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
      .filter((message) => normalizeRole(message.agent_role) === normalizeRole(role))
      .slice()
      .reverse()[0];

  const tasks: SubagentTask[] = [
    {
      role: "Kuromi Finance",
      status: statusFromInvestigation(detail?.investigation.status, insight.coverageStatus),
      task: `Dieu phoi luong phan tich cho ${insight.pair.symbol}`,
      note:
        latestMessage("kuromi")?.content ||
        latestMessage("coordinator")?.content ||
        `Kuromi se tong hop mission va spawn team dong khi can cho ${insight.pair.symbol}.`,
    },
  ];

  const collaboratorRoles = detail
    ? Array.from(
        new Set(
          detail.transcript
            .map((message) => normalizeRole(message.agent_role))
            .filter((role) => role && role !== "user" && !isKuromiRole(role)),
        ),
      )
    : [];

  if (!collaboratorRoles.length) {
    tasks.push({
      role: "Dynamic Team",
      status: statusFromInvestigation(detail?.investigation.status, insight.coverageStatus),
      task: "Spawn cac collaborator theo nhu cau",
      note:
        detail?.investigation.status === "completed"
          ? "Khong co collaborator runtime nao duoc luu trong transcript hien tai."
          : `Kuromi co the dung spawn_team de tao cac member chuyen trach cho ${insight.pair.symbol}.`,
    });
    return tasks;
  }

  return tasks.concat(
    collaboratorRoles.map((role) => ({
      role: titleFromRole(role),
      status: statusFromInvestigation(detail?.investigation.status, insight.coverageStatus),
      task: `Dong gop goc nhin chuyen mon cho ${insight.pair.symbol}`,
      note:
        latestMessage(role)?.content ||
        `Thanh vien runtime ${titleFromRole(role)} da tham gia phien trao doi noi bo cua Kuromi.`,
    })),
  );
}

function normalizeRole(role: string): string {
  return role.trim().toLowerCase();
}

function isKuromiRole(role: string): boolean {
  return ["kuromi", "kuromi_finance", "kuromi-finance", "coordinator"].includes(normalizeRole(role));
}

function statusFromInvestigation(
  investigationStatus: string | undefined,
  coverageStatus: PairCoverageStatus,
): SubagentTask["status"] {
  if (investigationStatus === "completed" || coverageStatus === "covered") return "completed";
  if (investigationStatus === "running" || coverageStatus === "running") return "running";
  if (investigationStatus === "queued" || coverageStatus === "queued") return "queued";
  return "awaiting";
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
      `Chua co du lieu AI cho ${pair.symbol}. Nen tao investigation moi de luu brief va theo doi snapshot hien co.`,
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
