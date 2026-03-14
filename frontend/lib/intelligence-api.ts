import {
  CreateSchedulePayload,
  DashboardResponse,
  ScheduleView,
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

export function fetchDashboard(): Promise<DashboardResponse> {
  return apiFetch<DashboardResponse>("/api/dashboard");
}

export function fetchAgentStatuses(): Promise<AgentStatusView[]> {
  return apiFetch<AgentStatusView[]>("/api/agents/status");
}

export function fetchSchedules(): Promise<ScheduleView[]> {
  return apiFetch<ScheduleView[]>("/api/schedules");
}

export function createSchedule(payload: CreateSchedulePayload): Promise<ScheduleView> {
  return apiFetch<ScheduleView>("/api/schedules", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}
