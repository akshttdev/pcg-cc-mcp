use axum::{
    Router,
    extract::State,
    response::Json as ResponseJson,
    routing::get,
};
use serde::Serialize;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_usage_percent: f32,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_usage_percent: f32,
    pub process_count: usize,
    pub uptime_seconds: u64,
    pub load_average: LoadAverage,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct LoadAverage {
    pub one_minute: f64,
    pub five_minutes: f64,
    pub fifteen_minutes: f64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct DetailedSystemMetrics {
    pub metrics: SystemMetrics,
    pub top_processes: Vec<ProcessInfo>,
    pub per_cpu_usage: Vec<f32>,
}

/// Get current system metrics
pub async fn get_system_metrics(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<SystemMetrics>>, ApiError> {
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    // Wait a bit for accurate CPU measurement
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_all();

    let disks = Disks::new_with_refreshed_list();

    // Calculate disk usage (sum all disks)
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(used, total), disk| {
        (
            used + (disk.total_space() - disk.available_space()),
            total + disk.total_space(),
        )
    });

    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    let cpu_usage = sys.global_cpu_usage();

    let load_avg = System::load_average();

    let metrics = SystemMetrics {
        cpu_usage_percent: cpu_usage,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        memory_usage_percent: if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        },
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        disk_usage_percent: if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        },
        process_count: sys.processes().len(),
        uptime_seconds: System::uptime(),
        load_average: LoadAverage {
            one_minute: load_avg.one,
            five_minutes: load_avg.five,
            fifteen_minutes: load_avg.fifteen,
        },
    };

    Ok(ResponseJson(ApiResponse::success(metrics)))
}

/// Get detailed system metrics including top processes
pub async fn get_detailed_metrics(
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<DetailedSystemMetrics>>, ApiError> {
    let mut sys = System::new_all();

    // Wait a bit for accurate CPU measurement
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();

    // Calculate disk usage
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(used, total), disk| {
        (
            used + (disk.total_space() - disk.available_space()),
            total + disk.total_space(),
        )
    });

    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();
    let cpu_usage = sys.global_cpu_usage();

    let load_avg = System::load_average();

    // Get per-CPU usage
    let per_cpu_usage: Vec<f32> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();

    // Get top 10 processes by CPU usage
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| ProcessInfo {
            name: p.name().to_string_lossy().to_string(),
            pid: p.pid().as_u32(),
            cpu_usage: p.cpu_usage(),
            memory_bytes: p.memory(),
        })
        .collect();

    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    let top_processes: Vec<ProcessInfo> = processes.into_iter().take(10).collect();

    let metrics = SystemMetrics {
        cpu_usage_percent: cpu_usage,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        memory_usage_percent: if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        },
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        disk_usage_percent: if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        },
        process_count: sys.processes().len(),
        uptime_seconds: System::uptime(),
        load_average: LoadAverage {
            one_minute: load_avg.one,
            five_minutes: load_avg.five,
            fifteen_minutes: load_avg.fifteen,
        },
    };

    Ok(ResponseJson(ApiResponse::success(DetailedSystemMetrics {
        metrics,
        top_processes,
        per_cpu_usage,
    })))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/system-metrics", get(get_system_metrics))
        .route("/system-metrics/detailed", get(get_detailed_metrics))
}
