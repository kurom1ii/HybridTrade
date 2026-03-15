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
  agent_role: string;
  message: string;
  last_status: string;
  last_result?: string | null;
  allowed_tools?: string[] | null;
  allowed_mcps?: string[] | null;
  skills?: string[] | null;
}

export interface AgentStatusView {
  role: string;
  label: string;
  status: string;
  last_seen_at?: string | null;
  last_message?: string | null;
  open_runs: number;
}

export interface DashboardResponse {
  agent_statuses: AgentStatusView[];
  schedules: ScheduleView[];
}

export interface CreateSchedulePayload {
  name: string;
  cron_expr: string;
  job_type: string;
  enabled: boolean;
  agent_role: string;
  message: string;
  payload?: Record<string, unknown>;
  allowed_tools?: string[] | null;
  allowed_mcps?: string[] | null;
  skills?: string[] | null;
}

export interface UpdateSchedulePayload {
  name?: string;
  cron_expr?: string;
  enabled?: boolean;
  agent_role?: string;
  message?: string;
  payload?: Record<string, unknown>;
  allowed_tools?: string[] | null;
  allowed_mcps?: string[] | null;
  skills?: string[] | null;
}

export interface CapabilitiesView {
  tools: string[];
  mcps: string[];
  skills: string[];
}

export interface InstrumentView {
  symbol: string;
  name: string;
  category: string;
  direction: string;
  confidence: number;
  price: number;
  change_pct: number;
  timeframe: string;
  analysis: string;
  key_levels: { price: number; label: string; type: string }[];
  updated_at: string;
}

export interface UpsertInstrumentPayload {
  name?: string;
  category?: string;
  direction?: string;
  confidence?: number;
  price?: number;
  change_pct?: number;
  timeframe?: string;
  analysis?: string;
  key_levels?: { price: number; label: string; type: string }[];
}

export interface ScheduleResultDetail {
  content: string;
  provider: string;
  model: string;
  tool_calls: ToolCallDetail[];
  system_prompt: string;
  available_tools: string[];
  history_count: number;
}

export interface ToolCallDetail {
  name: string;
  source: string;
  status: string;
  input: Record<string, unknown>;
  output_preview: string;
}
