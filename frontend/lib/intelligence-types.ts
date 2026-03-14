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
}
