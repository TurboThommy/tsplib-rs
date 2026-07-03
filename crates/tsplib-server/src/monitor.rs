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
