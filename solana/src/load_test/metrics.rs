//! Metrics and report structures for load testing.

use serde::{Deserialize, Serialize};

/// Per-transaction metrics collected during load testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TxMetrics {
    pub signature: String,
    pub submit_time_ms: u64,
    pub confirm_time_ms: Option<u64>,
    pub latency_ms: Option<u64>,
    pub compute_units: Option<u64>,
    pub slot: Option<u64>,
    pub success: bool,
    pub error: Option<String>,
}

/// Comprehensive load test report containing all metrics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct LoadTestReport {
    // Test configuration
    pub destination_chain: String,
    pub token_id: String,
    pub transfer_amount: String,
    pub duration_secs: u64,
    pub delay_ms: u64,
    pub contention_mode: String,
    pub num_keypairs: usize,

    // Throughput metrics
    pub total_submitted: u64,
    pub total_confirmed: u64,
    pub total_failed: u64,
    pub test_duration_secs: f64,
    pub tps_submitted: f64,
    pub tps_confirmed: f64,
    pub landing_rate: f64,

    // Solana metrics
    pub avg_latency_ms: Option<f64>,
    pub min_latency_ms: Option<u64>,
    pub max_latency_ms: Option<u64>,
    pub avg_compute_units: Option<f64>,
    pub min_compute_units: Option<u64>,
    pub max_compute_units: Option<u64>,

    // Verification results (populated after verify phase)
    pub verification: Option<VerificationReport>,

    // Individual transaction metrics
    pub transactions: Vec<TxMetrics>,
}

/// Report from transaction verification phase.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct VerificationReport {
    pub total_verified: u64,
    pub successful: u64,
    pub pending: u64,
    pub failed: u64,
    pub success_rate: f64,
    pub failure_reasons: Vec<FailureCategory>,
}

/// Categorized failure count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FailureCategory {
    pub reason: String,
    pub count: u64,
}
