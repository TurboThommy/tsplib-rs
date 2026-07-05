//! This module provides functionality for monitoring system memory usage and aborting a running TSP solver if memory usage exceeds a specified threshold.
//! It includes functions to calculate memory utilization, check for cgroup v2 limits on Linux, and spawn a background task that monitors memory usage at regular intervals.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use sysinfo::System;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Memory usage threshold for aborting a running TSP solver.
pub const MEMORY_ABORT_THRESHOLD: f64 = 0.95;

/// Interval for sampling memory usage.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

/// Returns the current memory utilization as a fraction of total memory.
///
/// # Arguments
/// * `sys` - A mutable reference to a `System` instance from the `sysinfo` crate, which is used to query system information.
///
/// # Returns
/// * `Option<f64>` - Returns Some(fraction) if memory utilization can be determined, or None if it cannot be determined (e.g., total memory is zero).
pub fn memory_utilization(sys: &mut System) -> Option<f64> {
    // prefer the container's own limit on linux
    // falls through on other platforms or whe no limit is configured
    #[cfg(target_os = "linux")]
    if let Some(utilization) = cgroup_v2_utilization() {
        return Some(utilization);
    }

    sys.refresh_memory();
    let total = sys.total_memory();
    if total == 0 {
        return None;
    }
    Some(total.saturating_sub(sys.available_memory()) as f64 / total as f64)
}

/// Returns the current memory utilization as a fraction of the cgroup v2 memory limit on Linux.
///
/// # Returns
/// * `Option<f64>` - Returns Some(fraction) if memory utilization can be determined,
///   or None if it cannot be determined (e.g., not running on Linux, cgroup v2 not in use, or memory limit is not set).
#[cfg(target_os = "linux")]
fn cgroup_v2_utilization() -> Option<f64> {
    let max_raw = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok()?;
    let max_raw = max_raw.trim();
    if max_raw == "max" {
        return None;
    }
    let max: u64 = max_raw.parse().ok()?;
    if max == 0 {
        return None;
    }
    let current: u64 = std::fs::read_to_string("/sys/fs/cgroup/memory.current")
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(current as f64 / max as f64)
}

/// Spawns a background task that monitors memory usage at regular intervals and cancels the provided `CancellationToken`
/// if memory utilization exceeds the specified threshold.
///
/// # Arguments
/// * `token` - A `CancellationToken` that will be cancelled if memory utilization exceeds the threshold.
/// * `threshold` - A fraction (between 0.0 and 1.0) representing the memory utilization threshold at which the task should cancel the token.
/// * `tripped` - An `Arc<AtomicBool>` that will be set to true if the memory threshold is exceeded and the token is cancelled.
///
/// # Returns
/// * `JoinHandle<()>` - A handle to the spawned task, which can be used to await its completion or cancel it if needed.
pub fn spawn_memory_guard(
    token: CancellationToken,
    threshold: f64,
    tripped: Arc<AtomicBool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // one reusable System instance; will be refreshed on each tick
        let mut sys = System::new();
        let mut ticker = tokio::time::interval(SAMPLE_INTERVAL);
        loop {
            ticker.tick().await;

            if token.is_cancelled() {
                break;
            }

            match memory_utilization(&mut sys) {
                Some(utilization) if utilization > threshold => {
                    tracing::warn!(
                        utilization = format!("{:.1}%", utilization * 100.0),
                        threshold = format!("{:.1}%", threshold * 100.0),
                        "Memory utilization exceeded threshold; aborting solver task."
                    );
                    tripped.store(true, Ordering::SeqCst);
                    token.cancel();
                    break;
                }
                Some(_) => {}
                None => {
                    tracing::warn!(
                        "Memory utilization unavailable; resource guard idle this tick."
                    );
                }
            }
        }
    })
}
