use std::env;
#[cfg(unix)]
use std::os::raw::{c_int, c_long};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::d3_live_compute_settlement::D3LiveComputeSettlementReport;
use crate::run_d3_live_compute_settlement;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L6StabilityReport {
    pub requested_cycles: usize,
    pub completed_cycles: usize,
    pub all_green: bool,
    pub cycle_delay_secs: u64,
    pub total_elapsed_ms: u128,
    pub p50_cycle_ms: Option<u128>,
    pub p99_cycle_ms: Option<u128>,
    pub initial_rss_kib: Option<i64>,
    pub final_rss_kib: Option<i64>,
    pub max_rss_kib: Option<i64>,
    pub rss_delta_kib: Option<i64>,
    pub cycles: Vec<L6CycleResult>,
}

impl L6StabilityReport {
    pub fn all_green(&self) -> bool {
        self.all_green
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# L6 Stability Driver\n\n");
        output.push_str("- requested_cycles: `");
        output.push_str(&self.requested_cycles.to_string());
        output.push_str("`\n");
        output.push_str("- completed_cycles: `");
        output.push_str(&self.completed_cycles.to_string());
        output.push_str("`\n");
        output.push_str("- cycle_delay_secs: `");
        output.push_str(&self.cycle_delay_secs.to_string());
        output.push_str("`\n");
        output.push_str("- total_elapsed_ms: `");
        output.push_str(&self.total_elapsed_ms.to_string());
        output.push_str("`\n");
        if let Some(p50) = self.p50_cycle_ms {
            output.push_str("- p50_cycle_ms: `");
            output.push_str(&p50.to_string());
            output.push_str("`\n");
        }
        if let Some(p99) = self.p99_cycle_ms {
            output.push_str("- p99_cycle_ms: `");
            output.push_str(&p99.to_string());
            output.push_str("`\n");
        }
        if let Some(max_rss) = self.max_rss_kib {
            output.push_str("- max_rss_kib: `");
            output.push_str(&max_rss.to_string());
            output.push_str("`\n");
        }
        if let Some(delta) = self.rss_delta_kib {
            output.push_str("- rss_delta_kib: `");
            output.push_str(&delta.to_string());
            output.push_str("`\n");
        }
        output.push('\n');

        output.push_str("## Result\n\n");
        output.push_str(if self.all_green {
            "L6 driver completed all requested cycles green. Each completed cycle passed D3 settlement validation.\n\n"
        } else {
            "L6 driver failed closed. See the first failed cycle below.\n\n"
        });

        output.push_str("## Cycles\n\n");
        for cycle in &self.cycles {
            output.push_str("- ");
            output.push_str(if cycle.green { "PASS" } else { "FAIL" });
            output.push_str(" cycle `");
            output.push_str(&cycle.index.to_string());
            output.push_str("`: elapsed_ms=`");
            output.push_str(&cycle.elapsed_ms.to_string());
            output.push('`');
            if let Some(job_id) = &cycle.job_id {
                output.push_str(" job_id=`");
                output.push_str(job_id);
                output.push('`');
            }
            if let Some(uuid_job_id) = &cycle.uuid_job_id {
                output.push_str(" uuid_job_id=`");
                output.push_str(uuid_job_id);
                output.push('`');
            }
            if let Some(settlement_status) = &cycle.settlement_status {
                output.push_str(" settlement_status=`");
                output.push_str(settlement_status);
                output.push('`');
            }
            if let Some(actual_cost) = cycle.actual_cost {
                output.push_str(" actual_cost=`");
                output.push_str(&actual_cost.to_string());
                output.push('`');
            }
            if let Some(error) = &cycle.error {
                output.push_str(" error=`");
                output.push_str(error);
                output.push('`');
            }
            output.push('\n');
        }

        output
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L6CycleResult {
    pub index: usize,
    pub green: bool,
    pub elapsed_ms: u128,
    pub provider_node_id: Option<String>,
    pub requester_node_id: Option<String>,
    pub tunnel_url: Option<String>,
    pub offer_id: Option<String>,
    pub job_id: Option<String>,
    pub uuid_job_id: Option<String>,
    pub settlement_id: Option<String>,
    pub settlement_status: Option<String>,
    pub actual_cost: Option<i64>,
    pub provider_payout: Option<i64>,
    pub requester_adjustment: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct L6Config {
    cycles: usize,
    cycle_delay_secs: u64,
}

pub fn run_l6_stability_driver() -> L6StabilityReport {
    let config = L6Config::from_env();
    run_l6_stability_driver_with_config(config)
}

fn run_l6_stability_driver_with_config(config: L6Config) -> L6StabilityReport {
    let started = Instant::now();
    let initial_rss = current_rss_kib();
    let mut max_rss = initial_rss;
    let mut cycles = Vec::new();

    for index in 1..=config.cycles {
        let cycle_started = Instant::now();
        let d3 = run_d3_live_compute_settlement();
        let elapsed_ms = cycle_started.elapsed().as_millis();
        let cycle = cycle_result_from_d3(index, elapsed_ms, d3);
        let green = cycle.green;
        cycles.push(cycle);

        if let Some(rss) = current_rss_kib() {
            max_rss = Some(max_rss.map_or(rss, |current| current.max(rss)));
        }

        if !green {
            break;
        }

        if index < config.cycles && config.cycle_delay_secs > 0 {
            thread::sleep(Duration::from_secs(config.cycle_delay_secs));
        }
    }

    let final_rss = current_rss_kib();
    if let Some(rss) = final_rss {
        max_rss = Some(max_rss.map_or(rss, |current| current.max(rss)));
    }
    let rss_delta = match (initial_rss, final_rss) {
        (Some(start), Some(end)) => Some(end - start),
        _ => None,
    };
    let mut elapsed_cycles = cycles
        .iter()
        .map(|cycle| cycle.elapsed_ms)
        .collect::<Vec<_>>();
    elapsed_cycles.sort_unstable();
    let all_green = cycles.len() == config.cycles && cycles.iter().all(|cycle| cycle.green);

    L6StabilityReport {
        requested_cycles: config.cycles,
        completed_cycles: cycles.len(),
        all_green,
        cycle_delay_secs: config.cycle_delay_secs,
        total_elapsed_ms: started.elapsed().as_millis(),
        p50_cycle_ms: percentile(&elapsed_cycles, 50),
        p99_cycle_ms: percentile(&elapsed_cycles, 99),
        initial_rss_kib: initial_rss,
        final_rss_kib: final_rss,
        max_rss_kib: max_rss,
        rss_delta_kib: rss_delta,
        cycles,
    }
}

impl L6Config {
    fn from_env() -> Self {
        let cycles = env::var("L6_CYCLES")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1);
        let cycle_delay_secs = env::var("L6_CYCLE_DELAY_SECS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(0);
        Self {
            cycles,
            cycle_delay_secs,
        }
    }
}

fn cycle_result_from_d3(
    index: usize,
    elapsed_ms: u128,
    d3: D3LiveComputeSettlementReport,
) -> L6CycleResult {
    let error = if d3.all_green() {
        None
    } else {
        d3.subtests
            .iter()
            .find_map(|subtest| match &subtest.status {
                crate::D3Status::Failed { reason } => {
                    Some(format!("{}: {}", subtest.name, trim_for_report(reason)))
                }
                crate::D3Status::Passed => None,
            })
    };
    L6CycleResult {
        index,
        green: d3.all_green(),
        elapsed_ms,
        provider_node_id: d3.provider_node_id,
        requester_node_id: d3.requester_node_id,
        tunnel_url: d3.tunnel_url,
        offer_id: d3.offer_id,
        job_id: d3.job_id,
        uuid_job_id: d3.uuid_job_id,
        settlement_id: d3.settlement_id,
        settlement_status: d3.settlement_status,
        actual_cost: d3.actual_cost,
        provider_payout: d3.provider_payout,
        requester_adjustment: d3.requester_adjustment,
        error,
    }
}

fn percentile(values: &[u128], percentile: u32) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    let percentile = percentile.min(100) as usize;
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values.get(index).copied()
}

fn current_rss_kib() -> Option<i64> {
    rusage_maxrss_kib()
}

#[cfg(not(unix))]
fn rusage_maxrss_kib() -> Option<i64> {
    None
}

#[cfg(unix)]
fn rusage_maxrss_kib() -> Option<i64> {
    let mut usage = RUsage::default();
    // SAFETY: getrusage writes into the provided RUsage buffer when called
    // with RUSAGE_SELF. The struct layout mirrors the platform C layout for
    // the fields up through ru_maxrss and keeps the trailing integer fields.
    let rc = unsafe { getrusage(RUSAGE_SELF, &mut usage as *mut RUsage) };
    if rc != 0 {
        return None;
    }
    let maxrss = usage.ru_maxrss;
    if maxrss < 0 {
        return None;
    }
    #[cfg(target_os = "macos")]
    {
        Some((maxrss / 1024).max(0))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(maxrss)
    }
}

#[cfg(all(unix, target_os = "macos"))]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TimeVal {
    tv_sec: c_long,
    tv_usec: c_int,
}

#[cfg(all(unix, not(target_os = "macos")))]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct TimeVal {
    tv_sec: c_long,
    tv_usec: c_long,
}

#[cfg(unix)]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct RUsage {
    ru_utime: TimeVal,
    ru_stime: TimeVal,
    ru_maxrss: c_long,
    ru_ixrss: c_long,
    ru_idrss: c_long,
    ru_isrss: c_long,
    ru_minflt: c_long,
    ru_majflt: c_long,
    ru_nswap: c_long,
    ru_inblock: c_long,
    ru_oublock: c_long,
    ru_msgsnd: c_long,
    ru_msgrcv: c_long,
    ru_nsignals: c_long,
    ru_nvcsw: c_long,
    ru_nivcsw: c_long,
}

#[cfg(unix)]
const RUSAGE_SELF: c_int = 0;

#[cfg(unix)]
unsafe extern "C" {
    fn getrusage(who: c_int, usage: *mut RUsage) -> c_int;
}

fn trim_for_report(input: &str) -> String {
    const MAX: usize = 240;
    let compact = input.replace('\n', " ");
    if compact.chars().count() <= MAX {
        return compact;
    }
    let mut output = compact.chars().take(MAX).collect::<String>();
    output.push_str("...");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_handles_small_samples() {
        assert_eq!(percentile(&[], 50), None);
        assert_eq!(percentile(&[10], 99), Some(10));
        assert_eq!(percentile(&[10, 20, 30], 50), Some(20));
        assert_eq!(percentile(&[10, 20, 30], 99), Some(30));
    }

    #[test]
    fn l6_markdown_surfaces_cycle_status() {
        let report = L6StabilityReport {
            requested_cycles: 1,
            completed_cycles: 1,
            all_green: true,
            cycle_delay_secs: 0,
            total_elapsed_ms: 42,
            p50_cycle_ms: Some(42),
            p99_cycle_ms: Some(42),
            initial_rss_kib: Some(100),
            final_rss_kib: Some(110),
            max_rss_kib: Some(110),
            rss_delta_kib: Some(10),
            cycles: vec![L6CycleResult {
                index: 1,
                green: true,
                elapsed_ms: 42,
                provider_node_id: None,
                requester_node_id: None,
                tunnel_url: None,
                offer_id: None,
                job_id: Some("playful/123/1".to_owned()),
                uuid_job_id: None,
                settlement_id: None,
                settlement_status: Some("settled".to_owned()),
                actual_cost: Some(2),
                provider_payout: Some(2),
                requester_adjustment: Some(0),
                error: None,
            }],
        };

        let markdown = report.to_markdown();

        assert!(markdown.contains("L6 driver completed all requested cycles green"));
        assert!(markdown.contains("job_id=`playful/123/1`"));
        assert!(markdown.contains("settlement_status=`settled`"));
    }

    #[test]
    fn rss_sampler_is_non_negative_when_available() {
        if let Some(rss) = current_rss_kib() {
            assert!(rss >= 0);
        }
    }
}
