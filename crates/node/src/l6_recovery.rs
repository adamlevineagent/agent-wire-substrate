//! L6 recovery primitives for the failure-injection seam.
//!
//! The policy below is intentionally small: it makes the substrate recovery
//! contract explicit at Newman's `RecoveryPolicy` hooks without coupling the
//! deterministic injection harness to live Wire services. The invariant is the
//! same one the live D3/L6 lane relies on: the canonical job ref is write-once
//! across claim, completion, and settlement, including tunnel rotation.

use agent_wire_foundation::CrossGraphRef;

use crate::l6_failure_injection::{
    run_failure_injection_scenarios_with_policy, InjectionReport, InjectionState, RecoveryPolicy,
};

#[derive(Debug, Default, Clone)]
pub struct SubstrateRecoveryPolicy;

impl RecoveryPolicy for SubstrateRecoveryPolicy {
    fn allow_claim(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        let job_key = canonical_job_ref(job_ref)?;
        if contains_job(&state.claimed, &job_key)? {
            return Err(format!(
                "job_ref={job_key:?} already has a provider claim; recovery rejects duplicate claim"
            ));
        }
        Ok(())
    }

    fn allow_completion(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        let job_key = canonical_job_ref(job_ref)?;
        if !contains_job(&state.claimed, &job_key)? {
            return Err(format!(
                "job_ref={job_key:?} cannot complete before a preserved claim exists"
            ));
        }
        if contains_job(&state.completed, &job_key)? {
            return Err(format!(
                "job_ref={job_key:?} already has a completion; recovery rejects duplicate completion"
            ));
        }
        Ok(())
    }

    fn allow_settlement(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        let job_key = canonical_job_ref(job_ref)?;
        if !contains_job(&state.completed, &job_key)? {
            return Err(format!(
                "job_ref={job_key:?} cannot settle before a preserved completion exists"
            ));
        }
        if contains_job(&state.settled, &job_key)? {
            return Err(format!(
                "job_ref={job_key:?} already has a settlement; recovery rejects duplicate settlement"
            ));
        }
        Ok(())
    }
}

pub fn run_l6_recovery_injection_scenarios() -> InjectionReport {
    run_failure_injection_scenarios_with_policy(&SubstrateRecoveryPolicy)
}

fn canonical_job_ref(job_ref: &str) -> Result<String, String> {
    job_ref
        .parse::<CrossGraphRef>()
        .map(|parsed| parsed.to_string())
        .map_err(|_| format!("job_ref={job_ref:?} is not a canonical cross-graph ref"))
}

fn contains_job(set: &std::collections::HashSet<String>, job_key: &str) -> Result<bool, String> {
    for existing in set {
        let existing_key = canonical_job_ref(existing)?;
        if existing_key == job_key {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substrate_policy_passes_all_injection_scenarios() {
        let report = run_l6_recovery_injection_scenarios();
        assert!(
            report.all_passed(),
            "substrate recovery policy must satisfy all kill points: {}",
            report.to_markdown()
        );
    }

    #[test]
    fn substrate_policy_rejects_malformed_job_refs() {
        let policy = SubstrateRecoveryPolicy;
        let state = InjectionState::new("https://tunnel-a.example");

        let result = policy.allow_claim(&state, "job/not-a-cross-graph-ref/1");

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .contains("not a canonical cross-graph ref"));
    }

    #[test]
    fn substrate_policy_keeps_claim_idempotent_across_tunnel_rotation() {
        let policy = SubstrateRecoveryPolicy;
        let mut state = InjectionState::new("https://tunnel-a.example");
        let job = "playful/123/l6-rotation/1";

        state.claim(&policy, job).unwrap();
        state.rotate_tunnel("https://tunnel-b.example");

        let duplicate = state.claim(&policy, job);

        assert!(duplicate.is_err());
        assert_eq!(state.claimed.len(), 1);
    }
}
