//! L6 observability layer — leaked-claim + orphan-settlement scanners,
//! throughput surface, RSS deviation tracking.
//!
//! Per /122/4 friction-sanding workstream + L6 stability dispatch
//! (`msg/partner-orchestrator/2026-05-04/dm-86cfda`). Newman's lane:
//! observability + failure injection. Kramer's lane: long-running driver
//! harness + tunnel rotation lifecycle + recovery semantics + settlement
//! consistency. Mesh intent declared at
//! `agent-wire-substrate/crates/node/src/l6_observability.rs+l6_failure_injection.rs`.
//!
//! This module is **purely additive on Kramer's `L6StabilityReport`** — it
//! takes the existing per-cycle results and applies invariant scans that
//! contradict-check the green status. A cycle that reports `green=true` but
//! has missing identifiers, missing settlement evidence, or duplicate
//! settlement IDs is a *leak* — the harness reported success but state is
//! inconsistent. Surfacing those at the observability layer keeps Kramer's
//! recovery primitives honest as he ships them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::l6_stability_driver::{L6CycleResult, L6StabilityReport};

/// One findings record per cycle. `green=true` cycles SHOULD produce
/// `findings` with all `kind` in `Pass`; any `Pass` other than `Pass` on a
/// green cycle is a substrate inconsistency that the L6 harness's surface
/// status missed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L6CycleObservability {
    pub index: usize,
    pub findings: Vec<ObservabilityFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityFinding {
    pub scan: ObservabilityScan,
    pub kind: ObservabilityKind,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservabilityScan {
    LeakedClaim,
    OrphanSettlement,
    DuplicateSettlement,
    SettlementShape,
    ThroughputAnomaly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservabilityKind {
    /// The scan ran and the invariant held.
    Pass,
    /// The scan ran and detected an invariant violation.
    Violation,
    /// The scan was skipped (cycle failed earlier; invariant N/A).
    SkippedCycleFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct L6ObservabilityReport {
    pub cycle_count: usize,
    /// Per-cycle invariant scans.
    pub cycles: Vec<L6CycleObservability>,
    /// Throughput in jobs-per-second across the whole run.
    /// `None` if no cycles completed or total elapsed was zero.
    pub jobs_per_second: Option<f64>,
    /// RSS delta from `initial_rss_kib` to `final_rss_kib` (KiB).
    /// Same number as `L6StabilityReport.rss_delta_kib`; surfaced here so
    /// observability consumers can reason about leak shape without crossing
    /// back to the driver report struct.
    pub rss_delta_kib: Option<i64>,
    /// `true` iff every per-cycle finding is `Pass` or `SkippedCycleFailed`.
    /// (A cycle that failed for harness reasons is not a leak; the leak
    /// scans only assert against `green=true` cycles.)
    pub all_invariants_held: bool,
}

impl L6ObservabilityReport {
    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# L6 Observability Report\n\n");
        output.push_str("- cycle_count: `");
        output.push_str(&self.cycle_count.to_string());
        output.push_str("`\n");
        if let Some(jps) = self.jobs_per_second {
            output.push_str(&format!("- jobs_per_second: `{jps:.4}`\n"));
        }
        if let Some(delta) = self.rss_delta_kib {
            output.push_str(&format!("- rss_delta_kib: `{delta}`\n"));
        }
        output.push_str("- all_invariants_held: `");
        output.push_str(&self.all_invariants_held.to_string());
        output.push_str("`\n\n## Per-cycle findings\n\n");
        for cycle in &self.cycles {
            output.push_str(&format!("### Cycle `{}`\n\n", cycle.index));
            for finding in &cycle.findings {
                output.push_str(&format!(
                    "- {:?}/{:?}: {}\n",
                    finding.scan, finding.kind, finding.detail
                ));
            }
            output.push('\n');
        }
        output
    }
}

/// Scan a finished L6 stability report for invariant violations and produce
/// the observability layer's report. Read-only — does not mutate anything,
/// does not call any external service. Suitable for inline use inside the
/// L6 driver's per-cycle hook OR after-the-fact analysis of a JSON-dumped
/// report from a long run.
pub fn observe_l6_stability(report: &L6StabilityReport) -> L6ObservabilityReport {
    let cycles: Vec<_> = report.cycles.iter().map(observe_single_cycle).collect();

    let mut all_held = true;
    for cycle in &cycles {
        for finding in &cycle.findings {
            if matches!(finding.kind, ObservabilityKind::Violation) {
                all_held = false;
            }
        }
    }

    // Run the cross-cycle duplicate-settlement scan; if it finds violations,
    // append a synthetic finding to each implicated cycle so the per-cycle
    // markdown surfaces it locally rather than only at the report level.
    let duplicates = scan_duplicate_settlements(&report.cycles);
    let mut cycles = cycles;
    for (idx, detail) in duplicates {
        if let Some(cycle) = cycles.iter_mut().find(|c| c.index == idx) {
            cycle.findings.push(ObservabilityFinding {
                scan: ObservabilityScan::DuplicateSettlement,
                kind: ObservabilityKind::Violation,
                detail,
            });
            all_held = false;
        }
    }

    L6ObservabilityReport {
        cycle_count: report.completed_cycles,
        cycles,
        jobs_per_second: compute_jobs_per_second(report.completed_cycles, report.total_elapsed_ms),
        rss_delta_kib: report.rss_delta_kib,
        all_invariants_held: all_held,
    }
}

/// Per-cycle invariant checks. Green cycles MUST have all settlement-shape
/// fields populated; missing fields on a green cycle are a leak.
fn observe_single_cycle(cycle: &L6CycleResult) -> L6CycleObservability {
    let mut findings = Vec::new();

    if !cycle.green {
        // The leak/orphan/shape scans are conditional on the cycle reporting
        // success; a failed cycle's missing fields are expected.
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::LeakedClaim,
            kind: ObservabilityKind::SkippedCycleFailed,
            detail: "cycle reported failure; leak scan does not apply".to_owned(),
        });
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::OrphanSettlement,
            kind: ObservabilityKind::SkippedCycleFailed,
            detail: "cycle reported failure; orphan scan does not apply".to_owned(),
        });
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::SettlementShape,
            kind: ObservabilityKind::SkippedCycleFailed,
            detail: "cycle reported failure; shape scan does not apply".to_owned(),
        });
        return L6CycleObservability {
            index: cycle.index,
            findings,
        };
    }

    // Leaked-claim scan: a green cycle must have a job_id (provider claimed)
    // AND a settlement_id (settlement was created). Missing either while the
    // cycle reports green is a contradiction.
    let leak_violations = leaked_claim_violations(cycle);
    if leak_violations.is_empty() {
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::LeakedClaim,
            kind: ObservabilityKind::Pass,
            detail: "job_id and settlement_id both populated on green cycle".to_owned(),
        });
    } else {
        for detail in leak_violations {
            findings.push(ObservabilityFinding {
                scan: ObservabilityScan::LeakedClaim,
                kind: ObservabilityKind::Violation,
                detail,
            });
        }
    }

    // Orphan-settlement scan: a settlement_id is present without a job_id
    // (settlement created but no matching claim) — orphan in the per-cycle
    // sense. Cross-cycle orphans (settlement that maps to a job from a
    // different cycle) are caught by the duplicate-settlement scan.
    let orphan_violations = orphan_settlement_violations(cycle);
    if orphan_violations.is_empty() {
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::OrphanSettlement,
            kind: ObservabilityKind::Pass,
            detail: "settlement_id is present iff job_id is present".to_owned(),
        });
    } else {
        for detail in orphan_violations {
            findings.push(ObservabilityFinding {
                scan: ObservabilityScan::OrphanSettlement,
                kind: ObservabilityKind::Violation,
                detail,
            });
        }
    }

    // Settlement-shape scan: green + settled cycles must report
    // settlement_status="settled", actual_cost > 0 (compute happened),
    // provider_payout > 0 (provider got paid). A green cycle with zero cost
    // or zero payout means D3 reported success but the economics are
    // suspect.
    let shape_violations = settlement_shape_violations(cycle);
    if shape_violations.is_empty() {
        findings.push(ObservabilityFinding {
            scan: ObservabilityScan::SettlementShape,
            kind: ObservabilityKind::Pass,
            detail: "settlement_status=settled, actual_cost>0, provider_payout>0".to_owned(),
        });
    } else {
        for detail in shape_violations {
            findings.push(ObservabilityFinding {
                scan: ObservabilityScan::SettlementShape,
                kind: ObservabilityKind::Violation,
                detail,
            });
        }
    }

    L6CycleObservability {
        index: cycle.index,
        findings,
    }
}

fn leaked_claim_violations(cycle: &L6CycleResult) -> Vec<String> {
    let mut out = Vec::new();
    if cycle.job_id.is_none() {
        out.push("green cycle has no job_id; provider claim path appears empty".to_owned());
    }
    if cycle.settlement_id.is_none() {
        out.push(
            "green cycle has no settlement_id; settlement was never recorded but cycle is green"
                .to_owned(),
        );
    }
    if cycle.provider_node_id.is_none() {
        out.push("green cycle has no provider_node_id; provider attribution is missing".to_owned());
    }
    if cycle.requester_node_id.is_none() {
        out.push(
            "green cycle has no requester_node_id; requester attribution is missing".to_owned(),
        );
    }
    out
}

fn orphan_settlement_violations(cycle: &L6CycleResult) -> Vec<String> {
    let mut out = Vec::new();
    let has_settlement = cycle.settlement_id.is_some();
    let has_job = cycle.job_id.is_some();
    if has_settlement && !has_job {
        out.push(
            "settlement_id present without job_id; orphan settlement reachable from this cycle"
                .to_owned(),
        );
    }
    out
}

fn settlement_shape_violations(cycle: &L6CycleResult) -> Vec<String> {
    let mut out = Vec::new();
    match cycle.settlement_status.as_deref() {
        Some("settled") => {}
        Some(other) => out.push(format!(
            "green cycle reports settlement_status={other:?}, expected settled"
        )),
        None => out.push("green cycle has no settlement_status field populated".to_owned()),
    }
    match cycle.actual_cost {
        Some(cost) if cost > 0 => {}
        Some(cost) => out.push(format!(
            "green cycle has actual_cost={cost}; expected > 0 (compute should incur cost)"
        )),
        None => out.push("green cycle has no actual_cost recorded".to_owned()),
    }
    match cycle.provider_payout {
        Some(payout) if payout > 0 => {}
        Some(payout) => out.push(format!(
            "green cycle has provider_payout={payout}; expected > 0 (provider should be paid)"
        )),
        None => out.push("green cycle has no provider_payout recorded".to_owned()),
    }
    out
}

/// Cross-cycle duplicate-settlement scan. If two cycles share the same
/// settlement_id, that's a substrate-level double-settlement which the L6
/// driver's per-cycle pass/fail status would not catch (each cycle looks
/// fine in isolation). Returns (cycle_index, detail) per implicated cycle.
fn scan_duplicate_settlements(cycles: &[L6CycleResult]) -> Vec<(usize, String)> {
    let mut by_id: HashMap<&str, Vec<usize>> = HashMap::new();
    for cycle in cycles {
        if let Some(id) = cycle.settlement_id.as_deref() {
            by_id.entry(id).or_default().push(cycle.index);
        }
    }
    let mut out = Vec::new();
    for (id, indices) in by_id {
        if indices.len() <= 1 {
            continue;
        }
        for &idx in &indices {
            out.push((
                idx,
                format!(
                    "settlement_id={id:?} appears across {} cycles {:?} — \
                     duplicate-settlement violation",
                    indices.len(),
                    indices
                ),
            ));
        }
    }
    out
}

fn compute_jobs_per_second(completed: usize, total_ms: u128) -> Option<f64> {
    if completed == 0 || total_ms == 0 {
        return None;
    }
    let secs = total_ms as f64 / 1000.0;
    Some(completed as f64 / secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn green_cycle(index: usize) -> L6CycleResult {
        L6CycleResult {
            index,
            green: true,
            elapsed_ms: 100,
            provider_node_id: Some("provider-1".to_owned()),
            requester_node_id: Some("requester-1".to_owned()),
            tunnel_url: Some("https://tunnel.example".to_owned()),
            offer_id: Some("offer-1".to_owned()),
            job_id: Some("job-1".to_owned()),
            uuid_job_id: Some("00000000-0000-0000-0000-000000000001".to_owned()),
            settlement_id: Some(format!("settlement-{index}")),
            settlement_status: Some("settled".to_owned()),
            actual_cost: Some(2),
            provider_payout: Some(2),
            requester_adjustment: Some(0),
            error: None,
        }
    }

    fn report_with(cycles: Vec<L6CycleResult>, total_ms: u128) -> L6StabilityReport {
        L6StabilityReport {
            requested_cycles: cycles.len(),
            completed_cycles: cycles.len(),
            all_green: cycles.iter().all(|c| c.green),
            cycle_delay_secs: 0,
            total_elapsed_ms: total_ms,
            p50_cycle_ms: None,
            p99_cycle_ms: None,
            initial_rss_kib: Some(100),
            final_rss_kib: Some(110),
            max_rss_kib: Some(110),
            rss_delta_kib: Some(10),
            cycles,
        }
    }

    #[test]
    fn clean_green_run_passes_all_invariants() {
        let report = report_with(vec![green_cycle(1), green_cycle(2)], 2000);
        let obs = observe_l6_stability(&report);
        assert!(obs.all_invariants_held);
        for cycle in &obs.cycles {
            for finding in &cycle.findings {
                assert!(matches!(finding.kind, ObservabilityKind::Pass));
            }
        }
        assert_eq!(obs.jobs_per_second, Some(1.0));
        assert_eq!(obs.rss_delta_kib, Some(10));
    }

    #[test]
    fn green_cycle_missing_settlement_id_flagged_as_leaked_claim() {
        let mut cycle = green_cycle(1);
        cycle.settlement_id = None;
        let report = report_with(vec![cycle], 1000);
        let obs = observe_l6_stability(&report);
        assert!(!obs.all_invariants_held);
        let cycle_findings = &obs.cycles[0].findings;
        assert!(cycle_findings.iter().any(|f| {
            f.scan == ObservabilityScan::LeakedClaim
                && f.kind == ObservabilityKind::Violation
                && f.detail.contains("no settlement_id")
        }));
    }

    #[test]
    fn green_cycle_with_settlement_but_no_job_id_flagged_as_orphan() {
        let mut cycle = green_cycle(1);
        cycle.job_id = None;
        let report = report_with(vec![cycle], 1000);
        let obs = observe_l6_stability(&report);
        assert!(!obs.all_invariants_held);
        let findings = &obs.cycles[0].findings;
        assert!(findings.iter().any(|f| {
            f.scan == ObservabilityScan::OrphanSettlement && f.kind == ObservabilityKind::Violation
        }));
        // Also flagged by leak scan because job_id is none.
        assert!(findings.iter().any(|f| {
            f.scan == ObservabilityScan::LeakedClaim && f.kind == ObservabilityKind::Violation
        }));
    }

    #[test]
    fn green_cycle_with_zero_provider_payout_flagged_as_settlement_shape() {
        let mut cycle = green_cycle(1);
        cycle.provider_payout = Some(0);
        let report = report_with(vec![cycle], 1000);
        let obs = observe_l6_stability(&report);
        assert!(!obs.all_invariants_held);
        assert!(obs.cycles[0].findings.iter().any(|f| {
            f.scan == ObservabilityScan::SettlementShape
                && f.kind == ObservabilityKind::Violation
                && f.detail.contains("provider_payout=0")
        }));
    }

    #[test]
    fn duplicate_settlement_id_across_cycles_flagged() {
        let mut a = green_cycle(1);
        let mut b = green_cycle(2);
        a.settlement_id = Some("settlement-shared".to_owned());
        b.settlement_id = Some("settlement-shared".to_owned());
        let report = report_with(vec![a, b], 2000);
        let obs = observe_l6_stability(&report);
        assert!(!obs.all_invariants_held);
        let count = obs
            .cycles
            .iter()
            .flat_map(|c| &c.findings)
            .filter(|f| f.scan == ObservabilityScan::DuplicateSettlement)
            .count();
        assert_eq!(count, 2, "both cycles should be flagged");
    }

    #[test]
    fn failed_cycle_skips_per_cycle_scans() {
        let cycle = L6CycleResult {
            index: 1,
            green: false,
            elapsed_ms: 100,
            provider_node_id: None,
            requester_node_id: None,
            tunnel_url: None,
            offer_id: None,
            job_id: None,
            uuid_job_id: None,
            settlement_id: None,
            settlement_status: None,
            actual_cost: None,
            provider_payout: None,
            requester_adjustment: None,
            error: Some("synthetic".to_owned()),
        };
        let report = report_with(vec![cycle], 100);
        let obs = observe_l6_stability(&report);
        // A failed cycle is not a leak; missing fields are expected.
        assert!(obs.all_invariants_held);
        for finding in &obs.cycles[0].findings {
            assert!(matches!(
                finding.kind,
                ObservabilityKind::SkippedCycleFailed
            ));
        }
    }

    #[test]
    fn jobs_per_second_handles_edge_cases() {
        assert_eq!(compute_jobs_per_second(0, 1000), None);
        assert_eq!(compute_jobs_per_second(5, 0), None);
        assert_eq!(compute_jobs_per_second(2, 1000), Some(2.0));
    }

    #[test]
    fn markdown_surfaces_all_invariants_status() {
        let report = report_with(vec![green_cycle(1)], 1000);
        let obs = observe_l6_stability(&report);
        let md = obs.to_markdown();
        assert!(md.contains("L6 Observability Report"));
        assert!(md.contains("all_invariants_held: `true`"));
        assert!(md.contains("Cycle `1`"));
    }
}
