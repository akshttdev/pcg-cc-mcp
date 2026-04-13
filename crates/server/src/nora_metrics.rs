//! Prometheus metrics for Nora

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Encoder, Gauge,
    HistogramVec, TextEncoder,
};

lazy_static! {
    /// Total number of Nora requests by type
    pub static ref NORA_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "nora_requests_total",
        "Total number of requests to Nora by request type",
        &["request_type", "priority"]
    )
    .unwrap();

    /// Total number of LLM calls
    pub static ref NORA_LLM_CALLS_TOTAL: CounterVec = register_counter_vec!(
        "nora_llm_calls_total",
        "Total number of LLM API calls",
        &["provider", "model", "status"]
    )
    .unwrap();

    /// Total number of TTS (text-to-speech) calls
    pub static ref NORA_TTS_CALLS_TOTAL: CounterVec = register_counter_vec!(
        "nora_tts_calls_total",
        "Total number of TTS API calls",
        &["provider", "status"]
    )
    .unwrap();

    /// Total number of STT (speech-to-text) calls
    pub static ref NORA_STT_CALLS_TOTAL: CounterVec = register_counter_vec!(
        "nora_stt_calls_total",
        "Total number of STT API calls",
        &["provider", "status"]
    )
    .unwrap();

    /// Number of active coordinated agents
    pub static ref NORA_ACTIVE_AGENTS: Gauge = register_gauge!(
        "nora_active_agents",
        "Number of active coordinated agents"
    )
    .unwrap();

    /// Request processing duration in seconds
    pub static ref NORA_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "nora_request_duration_seconds",
        "Duration of Nora request processing in seconds",
        &["request_type"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
    )
    .unwrap();

    /// LLM request duration in seconds
    pub static ref NORA_LLM_DURATION: HistogramVec = register_histogram_vec!(
        "nora_llm_duration_seconds",
        "Duration of LLM API calls in seconds",
        &["provider", "model"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    )
    .unwrap();

    /// Voice API duration in seconds
    pub static ref NORA_VOICE_DURATION: HistogramVec = register_histogram_vec!(
        "nora_voice_duration_seconds",
        "Duration of voice API calls in seconds",
        &["operation", "provider"],
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0]
    )
    .unwrap();

    /// Task creation total
    pub static ref NORA_TASKS_CREATED_TOTAL: CounterVec = register_counter_vec!(
        "nora_tasks_created_total",
        "Total number of tasks created by Nora",
        &["priority", "status"]
    )
    .unwrap();

    /// Coordination events total
    pub static ref NORA_COORDINATION_EVENTS_TOTAL: CounterVec = register_counter_vec!(
        "nora_coordination_events_total",
        "Total number of coordination events",
        &["event_type"]
    )
    .unwrap();

    /// Nora active status (1 = active, 0 = inactive)
    pub static ref NORA_ACTIVE_STATUS: Gauge = register_gauge!(
        "nora_active_status",
        "Nora active status (1 = active, 0 = inactive)"
    )
    .unwrap();

    /// LLM cache hits total (gauge updated from cache stats)
    pub static ref NORA_CACHE_HITS: Gauge = register_gauge!(
        "nora_cache_hits",
        "Total number of LLM cache hits"
    )
    .unwrap();

    /// LLM cache misses total (gauge updated from cache stats)
    pub static ref NORA_CACHE_MISSES: Gauge = register_gauge!(
        "nora_cache_misses",
        "Total number of LLM cache misses"
    )
    .unwrap();

    /// LLM cache hit rate (0.0 to 1.0)
    pub static ref NORA_CACHE_HIT_RATE: Gauge = register_gauge!(
        "nora_cache_hit_rate",
        "LLM cache hit rate (hits / total requests)"
    )
    .unwrap();

    /// LLM cache entry count
    pub static ref NORA_CACHE_ENTRIES: Gauge = register_gauge!(
        "nora_cache_entries",
        "Number of entries in the LLM cache"
    )
    .unwrap();
}

/// Export all Nora metrics in Prometheus text format
pub fn export_metrics() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

/// Record a Nora request
pub fn record_request(request_type: &str, priority: &str) {
    NORA_REQUESTS_TOTAL
        .with_label_values(&[request_type, priority])
        .inc();
}

/// Record LLM call
pub fn record_llm_call(provider: &str, model: &str, status: &str, duration_secs: f64) {
    NORA_LLM_CALLS_TOTAL
        .with_label_values(&[provider, model, status])
        .inc();
    NORA_LLM_DURATION
        .with_label_values(&[provider, model])
        .observe(duration_secs);
}

/// Record TTS call
pub fn record_tts_call(provider: &str, status: &str, duration_secs: f64) {
    NORA_TTS_CALLS_TOTAL
        .with_label_values(&[provider, status])
        .inc();
    NORA_VOICE_DURATION
        .with_label_values(&["tts", provider])
        .observe(duration_secs);
}

/// Record STT call
pub fn record_stt_call(provider: &str, status: &str, duration_secs: f64) {
    NORA_STT_CALLS_TOTAL
        .with_label_values(&[provider, status])
        .inc();
    NORA_VOICE_DURATION
        .with_label_values(&["stt", provider])
        .observe(duration_secs);
}

/// Update active agents count
pub fn set_active_agents(count: usize) {
    NORA_ACTIVE_AGENTS.set(count as f64);
}

/// Record task creation
pub fn record_task_created(priority: &str, status: &str) {
    NORA_TASKS_CREATED_TOTAL
        .with_label_values(&[priority, status])
        .inc();
}

/// Record coordination event
pub fn record_coordination_event(event_type: &str) {
    NORA_COORDINATION_EVENTS_TOTAL
        .with_label_values(&[event_type])
        .inc();
}

/// Set Nora active status
pub fn set_nora_active(active: bool) {
    NORA_ACTIVE_STATUS.set(if active { 1.0 } else { 0.0 });
}

/// Update cache metrics from cache stats
pub fn update_cache_metrics(stats: &nora::cache::CacheStats) {
    NORA_CACHE_HITS.set(stats.hits as f64);
    NORA_CACHE_MISSES.set(stats.misses as f64);
    NORA_CACHE_HIT_RATE.set(stats.hit_rate);
    NORA_CACHE_ENTRIES.set(stats.entry_count as f64);
}

/// Start a request timer
pub fn start_request_timer(request_type: &str) -> prometheus::HistogramTimer {
    NORA_REQUEST_DURATION
        .with_label_values(&[request_type])
        .start_timer()
}
