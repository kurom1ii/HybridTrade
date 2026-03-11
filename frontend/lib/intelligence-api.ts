import {
  CreateInvestigationPayload,
  DashboardResponse,
  InvestigationDetail,
  InvestigationSummary,
  ScheduleView,
  HeartbeatView,
  AgentStatusView,
} from "@/lib/intelligence-types";

const rawBaseUrl = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";
export const API_BASE_URL = rawBaseUrl.replace(/\/$/, "");

export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    cache: "no-store",
  });

  if (!response.ok) {
    const text = await response.text();
    throw new ApiError(text || `Request failed with status ${response.status}`, response.status);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}

export function investigationStreamUrl(investigationId: string): string {
  return `${API_BASE_URL}/api/investigations/${investigationId}/stream`;
}

export function fetchDashboard(): Promise<DashboardResponse> {
  return apiFetch<DashboardResponse>("/api/dashboard");
}

export function fetchInvestigations(): Promise<InvestigationSummary[]> {
  return apiFetch<InvestigationSummary[]>("/api/investigations");
}

export function fetchInvestigation(investigationId: string): Promise<InvestigationDetail> {
  return apiFetch<InvestigationDetail>(`/api/investigations/${investigationId}`);
}

export function createInvestigation(payload: CreateInvestigationPayload): Promise<InvestigationDetail> {
  return apiFetch<InvestigationDetail>("/api/investigations", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function fetchAgentStatuses(): Promise<AgentStatusView[]> {
  return apiFetch<AgentStatusView[]>("/api/agents/status");
}

export function fetchHeartbeats(): Promise<HeartbeatView[]> {
  return apiFetch<HeartbeatView[]>("/api/heartbeats");
}

export function fetchSchedules(): Promise<ScheduleView[]> {
  return apiFetch<ScheduleView[]>("/api/schedules");
}
