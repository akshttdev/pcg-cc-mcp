import { useQuery } from '@tanstack/react-query';

export interface SystemMetrics {
  cpu_usage_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  memory_usage_percent: number;
  disk_used_bytes: number;
  disk_total_bytes: number;
  disk_usage_percent: number;
  process_count: number;
  uptime_seconds: bigint;
  load_average: LoadAverage;
}

export interface LoadAverage {
  one_minute: number;
  five_minutes: number;
  fifteen_minutes: number;
}

export interface ProcessInfo {
  name: string;
  pid: number;
  cpu_usage: number;
  memory_bytes: number;
}

export interface DetailedSystemMetrics {
  metrics: SystemMetrics;
  top_processes: ProcessInfo[];
  per_cpu_usage: number[];
}

async function fetchSystemMetrics(): Promise<SystemMetrics> {
  const res = await fetch('/api/system-metrics');
  if (!res.ok) {
    throw new Error('Failed to fetch system metrics');
  }
  const data = await res.json();
  return data.data;
}

async function fetchDetailedMetrics(): Promise<DetailedSystemMetrics> {
  const res = await fetch('/api/system-metrics/detailed');
  if (!res.ok) {
    throw new Error('Failed to fetch detailed metrics');
  }
  const data = await res.json();
  return data.data;
}

export function useSystemMetrics(options?: { refetchInterval?: number }) {
  return useQuery({
    queryKey: ['system-metrics'],
    queryFn: fetchSystemMetrics,
    refetchInterval: options?.refetchInterval ?? 5000, // Default 5 second refresh
  });
}

export function useDetailedSystemMetrics(options?: { refetchInterval?: number }) {
  return useQuery({
    queryKey: ['system-metrics-detailed'],
    queryFn: fetchDetailedMetrics,
    refetchInterval: options?.refetchInterval ?? 5000,
  });
}
