export interface Citation {
  source_id: string;
  url: string;
  title: string;
  snippet: string;
}

export interface InvestigationSummary {
  id: string;
  topic: string;
  goal: string;
  status: string;
  source_scope: string;
  priority: string;
  summary?: string | null;
  final_report?: string | null;
  tags: string[];
  seed_urls: string[];
  created_at: string;
  updated_at: string;
  completed_at?: string | null;
}

export interface SectionView {
  id: string;
  slug: string;
  title: string;
  status: string;
  conclusion?: string | null;
  position: number;
  updated_at: string;
}

export interface MessageView {
  id: string;
  section_id?: string | null;
  agent_role: string;
  target_role?: string | null;
  kind: string;
  content: string;
  citations: Citation[];
  confidence?: number | null;
  created_at: string;
}

export interface FindingView {
  id: string;
  section_id?: string | null;
  agent_role: string;
  kind: string;
  title: string;
  summary: string;
  direction?: string | null;
  confidence: number;
  evidence: Citation[];
  created_at: string;
}

export interface SourceView {
  id: string;
  url: string;
  title: string;
  fetched_at: string;
  excerpt?: string | null;
  metadata: Record<string, unknown>;
}

export interface HeartbeatView {
  component: string;
  scope: string;
  status_text: string;
  health: string;
  last_seen_at: string;
  ttl_seconds: number;
  details: Record<string, unknown>;
}

export interface ScheduleView {
  id: string;
  name: string;
  cron_expr: string;
  job_type: string;
  enabled: boolean;
  payload: Record<string, unknown>;
  last_run_at?: string | null;
  next_run_at?: string | null;
  updated_at: string;
}

export interface AgentStatusView {
  role: string;
  label: string;
  status: string;
  last_seen_at?: string | null;
  last_message?: string | null;
  open_runs: number;
}

export interface DashboardStats {
  total_investigations: number;
  running_investigations: number;
  completed_investigations: number;
  recent_findings: number;
  stale_heartbeats: number;
}

export interface DashboardResponse {
  stats: DashboardStats;
  recent_investigations: InvestigationSummary[];
  recent_findings: FindingView[];
  agent_statuses: AgentStatusView[];
  schedules: ScheduleView[];
  heartbeats: HeartbeatView[];
}

export interface InvestigationDetail {
  investigation: InvestigationSummary;
  sections: SectionView[];
  transcript: MessageView[];
  findings: FindingView[];
  sources: SourceView[];
  heartbeats: HeartbeatView[];
}

export interface CreateInvestigationPayload {
  topic: string;
  goal?: string;
  sections?: string[];
  source_scope?: string;
  priority?: string;
  tags?: string[];
  seed_urls?: string[];
}

export interface CreateSchedulePayload {
  name: string;
  cron_expr: string;
  job_type: string;
  enabled: boolean;
  payload?: Record<string, unknown>;
}

export interface AppStreamEvent<T = unknown> {
  event_type: string;
  investigation_id?: string | null;
  payload: T;
  timestamp: string;
}
