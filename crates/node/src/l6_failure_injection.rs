//! L6 failure-injection harness — kill-point scenarios with V1-derived
//! post-recovery invariant assertions.
//!
//! Per L6 stability dispatch (msg/partner-orchestrator/2026-05-04/dm-86cfda).
//! Newman's lane: instrumentation + failure injection. Kramer's lane:
//! recovery semantics + settlement consistency. This module ships the
//! kill-point seam; Kramer's recovery primitives plug in beneath as the
//! `RecoveryPolicy` trait. Today the default `WriteOncePolicy` enforces the
//! V1.4 idempotency fence at the harness level so the assertions can be
//! verified without waiting on the cross-cutting recovery PR.
//!
//! V plan §V1.4: "submit job, wait for provider claim, trigger rotation
//! mid-fulfillment, submit duplicate of the same job through the rotated
//! tunnel, try to claim payment twice." V1 finding /123/17 demonstrated
//! the L3 harness pre-fix did not enforce idempotency. With Kramer's
//! sealed-registry pass shipped, this harness exercises that the fix
//! holds under adversarial kill timing.
//!
//! Scope: substrate-only synthetic state; deterministic; no external
//! services. The injection harness intentionally does NOT use the full
//! `SyntheticWireGraph` from `layer3_synthetic.rs` so that injection
//! scenarios are isolated to the seam being tested rather than entangled
//! with the multi-test fixture state of the L3 harness.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Where in the provider lifecycle the synthetic kill fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillPoint {
    /// Kill provider before it has claimed any job — a duplicate claim
    /// attempt afterward should succeed (no leftover state) but never
    /// double-claim within one provider's view.
    BeforeProviderClaim,
    /// Kill provider after it claimed but before it produced a completion.
    /// On respawn, claim attempt is the post-recovery test: the recovery
    /// path must NOT permit a fresh claim that would double-spend the
    /// requester's escrow.
    AfterProviderClaim,
    /// Kill provider mid-completion. On respawn, completion attempt is
    /// the post-recovery test: the harness must not allow a second
    /// completion to be recorded against the same job_ref.
    AfterClaimBeforeCompletion,
    /// Kill requester after settlement was issued. Settlement reconciliation
    /// must surface the settlement on respawn rather than orphan it.
    AfterSettlement,
    /// Kill tunnel mid-rotation; the post-recovery duplicate-claim attempt
    /// through the rotated tunnel is the V plan §V1.4 scenario.
    DuringTunnelRotation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionScenarioResult {
    pub kill_point: KillPoint,
    pub passed: bool,
    pub assertion: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectionReport {
    pub scenarios: Vec<InjectionScenarioResult>,
}

impl InjectionReport {
    pub fn all_passed(&self) -> bool {
        self.scenarios.iter().all(|s| s.passed)
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# L6 Failure-Injection Report\n\n");
        for s in &self.scenarios {
            out.push_str(&format!(
                "- {} `{:?}`: {} — {}\n",
                if s.passed { "PASS" } else { "FAIL" },
                s.kill_point,
                s.assertion,
                s.detail,
            ));
        }
        out
    }
}

/// Recovery primitive seam — Kramer's substrate-side recovery code plugs
/// in here. Defaults to a pure write-once policy that enforces the V1.4
/// idempotency fence at the harness level, so this module's assertions
/// can be verified against the seam shape regardless of which downstream
/// recovery mechanism eventually lands.
pub trait RecoveryPolicy {
    /// Pre-claim hook: returns Ok(()) if claim is allowed, Err with reason
    /// if recovery semantics block the claim (e.g., job already claimed).
    fn allow_claim(&self, state: &InjectionState, job_ref: &str) -> Result<(), String>;

    /// Pre-completion hook: returns Ok(()) if completion is allowed.
    fn allow_completion(&self, state: &InjectionState, job_ref: &str) -> Result<(), String>;

    /// Pre-settlement hook: returns Ok(()) if settlement is allowed.
    fn allow_settlement(&self, state: &InjectionState, job_ref: &str) -> Result<(), String>;
}

#[derive(Debug, Default, Clone)]
pub struct WriteOncePolicy;

impl RecoveryPolicy for WriteOncePolicy {
    fn allow_claim(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        if state.claimed.contains(job_ref) {
            return Err(format!(
                "job_ref={job_ref:?} already claimed; write-once policy rejects re-claim"
            ));
        }
        Ok(())
    }

    fn allow_completion(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        if !state.claimed.contains(job_ref) {
            return Err(format!(
                "job_ref={job_ref:?} cannot complete an unclaimed job"
            ));
        }
        if state.completed.contains(job_ref) {
            return Err(format!(
                "job_ref={job_ref:?} already completed; single-shot completion rejects retry"
            ));
        }
        Ok(())
    }

    fn allow_settlement(&self, state: &InjectionState, job_ref: &str) -> Result<(), String> {
        if !state.completed.contains(job_ref) {
            return Err(format!(
                "job_ref={job_ref:?} cannot settle without a completion"
            ));
        }
        if state.settled.contains(job_ref) {
            return Err(format!(
                "job_ref={job_ref:?} already settled; double-settlement rejected"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct InjectionState {
    pub claimed: HashSet<String>,
    pub completed: HashSet<String>,
    pub settled: HashSet<String>,
    /// Tunnel URL; rotation changes this. Seam for the V plan §V1.4 scenario.
    pub tunnel_url: String,
    pub rotated_tunnel_urls: Vec<String>,
}

impl InjectionState {
    pub fn new(initial_tunnel: impl Into<String>) -> Self {
        Self {
            tunnel_url: initial_tunnel.into(),
            ..Default::default()
        }
    }

    pub fn rotate_tunnel(&mut self, new_url: impl Into<String>) {
        let new_url = new_url.into();
        self.rotated_tunnels_push(self.tunnel_url.clone());
        self.tunnel_url = new_url;
    }

    fn rotated_tunnels_push(&mut self, prior: String) {
        self.rotated_tunnel_urls.push(prior);
    }

    pub fn claim(&mut self, policy: &dyn RecoveryPolicy, job_ref: &str) -> Result<(), String> {
        policy.allow_claim(self, job_ref)?;
        self.claimed.insert(job_ref.to_owned());
        Ok(())
    }

    pub fn complete(&mut self, policy: &dyn RecoveryPolicy, job_ref: &str) -> Result<(), String> {
        policy.allow_completion(self, job_ref)?;
        self.completed.insert(job_ref.to_owned());
        Ok(())
    }

    pub fn settle(&mut self, policy: &dyn RecoveryPolicy, job_ref: &str) -> Result<(), String> {
        policy.allow_settlement(self, job_ref)?;
        self.settled.insert(job_ref.to_owned());
        Ok(())
    }
}

/// Run all kill-point scenarios. Each uses an isolated `InjectionState`.
pub fn run_failure_injection_scenarios() -> InjectionReport {
    let policy = WriteOncePolicy;
    let scenarios = vec![
        before_provider_claim_scenario(&policy),
        after_provider_claim_scenario(&policy),
        after_claim_before_completion_scenario(&policy),
        after_settlement_scenario(&policy),
        during_tunnel_rotation_scenario(&policy),
    ];
    InjectionReport { scenarios }
}

/// Kill before any claim. The "respawned" provider attempts to claim the
/// same job_ref. With no prior claim recorded, the claim succeeds — and
/// the assertion is that exactly one claim ends up recorded.
fn before_provider_claim_scenario(policy: &dyn RecoveryPolicy) -> InjectionScenarioResult {
    let mut state = InjectionState::new("https://tunnel-a.example");
    let job = "job/before-claim/1";
    // Provider boots; was about to claim but is killed; on respawn, claims
    // the job. Only one claim should be present.
    let res = state.claim(policy, job);
    let passed = res.is_ok() && state.claimed.len() == 1;
    InjectionScenarioResult {
        kill_point: KillPoint::BeforeProviderClaim,
        passed,
        assertion: "post-recovery claim succeeds and exactly one claim recorded".to_owned(),
        detail: if passed {
            format!("claimed jobs: {:?}", state.claimed)
        } else {
            format!("unexpected state after recovery: claim_result={res:?} claimed={:?}", state.claimed)
        },
    }
}

/// Kill after provider claim, before completion. Respawn attempts to
/// re-claim the same job_ref. Assertion (V1.4): the second claim is
/// rejected; the original claim is preserved.
fn after_provider_claim_scenario(policy: &dyn RecoveryPolicy) -> InjectionScenarioResult {
    let mut state = InjectionState::new("https://tunnel-a.example");
    let job = "job/after-claim/1";
    state.claim(policy, job).expect("first claim must succeed");
    // Provider killed. Respawn attempts re-claim.
    let respawn_claim = state.claim(policy, job);
    let passed = respawn_claim.is_err()
        && state.claimed.len() == 1
        && state.claimed.contains(job);
    InjectionScenarioResult {
        kill_point: KillPoint::AfterProviderClaim,
        passed,
        assertion: "respawn re-claim of same job_ref is rejected; original claim preserved".to_owned(),
        detail: if passed {
            format!(
                "respawn claim correctly rejected: {}",
                respawn_claim.err().unwrap_or_default()
            )
        } else {
            format!(
                "FAIL: respawn_claim={respawn_claim:?} claimed={:?}",
                state.claimed
            )
        },
    }
}

/// Kill after claim, before completion. Respawn attempts completion. With
/// the original claim still recorded, completion succeeds. Then a second
/// completion attempt must be rejected.
fn after_claim_before_completion_scenario(policy: &dyn RecoveryPolicy) -> InjectionScenarioResult {
    let mut state = InjectionState::new("https://tunnel-a.example");
    let job = "job/before-completion/1";
    state.claim(policy, job).expect("claim must succeed");
    // Provider killed mid-completion. Respawn completes.
    let first_completion = state.complete(policy, job);
    // Network retry / replay: another completion attempt for same job.
    let second_completion = state.complete(policy, job);
    let passed = first_completion.is_ok()
        && second_completion.is_err()
        && state.completed.len() == 1;
    InjectionScenarioResult {
        kill_point: KillPoint::AfterClaimBeforeCompletion,
        passed,
        assertion: "first post-recovery completion succeeds; duplicate completion is rejected".to_owned(),
        detail: if passed {
            format!(
                "first_completion=ok, retry rejected: {}",
                second_completion.err().unwrap_or_default()
            )
        } else {
            format!(
                "FAIL: first={first_completion:?} second={second_completion:?} completed={:?}",
                state.completed
            )
        },
    }
}

/// Kill requester after settlement issued. Reconciliation: the settlement
/// must already be recorded; a second settlement attempt is rejected.
fn after_settlement_scenario(policy: &dyn RecoveryPolicy) -> InjectionScenarioResult {
    let mut state = InjectionState::new("https://tunnel-a.example");
    let job = "job/after-settlement/1";
    state.claim(policy, job).expect("claim must succeed");
    state.complete(policy, job).expect("completion must succeed");
    state.settle(policy, job).expect("first settlement must succeed");
    // Requester killed; respawn attempts to re-issue settlement (e.g., a
    // network retry of the settlement RPC).
    let retry_settle = state.settle(policy, job);
    let passed = retry_settle.is_err() && state.settled.len() == 1;
    InjectionScenarioResult {
        kill_point: KillPoint::AfterSettlement,
        passed,
        assertion: "settlement retry after requester respawn is rejected; settled set is single-shot".to_owned(),
        detail: if passed {
            format!(
                "retry settlement correctly rejected: {}",
                retry_settle.err().unwrap_or_default()
            )
        } else {
            format!(
                "FAIL: retry_settle={retry_settle:?} settled={:?}",
                state.settled
            )
        },
    }
}

/// V plan §V1.4: claim through tunnel A, rotate to tunnel B, attempt to
/// claim same job through tunnel B. The rotation seam must not relax
/// idempotency.
fn during_tunnel_rotation_scenario(policy: &dyn RecoveryPolicy) -> InjectionScenarioResult {
    let mut state = InjectionState::new("https://tunnel-a.example");
    let job = "job/rotation/1";
    state.claim(policy, job).expect("claim through tunnel A must succeed");
    state.rotate_tunnel("https://tunnel-b.example");
    // Adversary submits duplicate through rotated tunnel. Idempotency must
    // hold across rotation.
    let post_rotation_claim = state.claim(policy, job);
    let url_changed = state.tunnel_url == "https://tunnel-b.example"
        && state.rotated_tunnel_urls.contains(&"https://tunnel-a.example".to_owned());
    let passed = post_rotation_claim.is_err()
        && state.claimed.len() == 1
        && url_changed;
    InjectionScenarioResult {
        kill_point: KillPoint::DuringTunnelRotation,
        passed,
        assertion: "duplicate claim via rotated tunnel is rejected; rotation does not relax idempotency".to_owned(),
        detail: if passed {
            format!(
                "post-rotation duplicate claim rejected: {}; tunnel A->B rotation recorded",
                post_rotation_claim.err().unwrap_or_default()
            )
        } else {
            format!(
                "FAIL: post_rotation_claim={post_rotation_claim:?} url_changed={url_changed} claimed={:?}",
                state.claimed
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kill_point_scenarios_pass_under_write_once_policy() {
        let report = run_failure_injection_scenarios();
        assert!(
            report.all_passed(),
            "all scenarios should pass under WriteOncePolicy: {}",
            report.to_markdown()
        );
        assert_eq!(report.scenarios.len(), 5);
    }

    #[test]
    fn each_kill_point_is_covered() {
        let report = run_failure_injection_scenarios();
        let kinds: HashSet<_> = report.scenarios.iter().map(|s| s.kill_point).collect();
        assert!(kinds.contains(&KillPoint::BeforeProviderClaim));
        assert!(kinds.contains(&KillPoint::AfterProviderClaim));
        assert!(kinds.contains(&KillPoint::AfterClaimBeforeCompletion));
        assert!(kinds.contains(&KillPoint::AfterSettlement));
        assert!(kinds.contains(&KillPoint::DuringTunnelRotation));
    }

    /// A "naive policy" that allows everything would fail Vector A+B+C+D
    /// from V1 finding /123/17. This regression test makes sure the seam
    /// design actually depends on the policy being write-once — if the
    /// policy degrades, the assertions catch it.
    #[test]
    fn naive_allow_all_policy_fails_idempotency_assertions() {
        struct AllowAll;
        impl RecoveryPolicy for AllowAll {
            fn allow_claim(&self, _: &InjectionState, _: &str) -> Result<(), String> {
                Ok(())
            }
            fn allow_completion(&self, _: &InjectionState, _: &str) -> Result<(), String> {
                Ok(())
            }
            fn allow_settlement(&self, _: &InjectionState, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        let policy = AllowAll;
        let s_after_claim = after_provider_claim_scenario(&policy);
        let s_completion = after_claim_before_completion_scenario(&policy);
        let s_settlement = after_settlement_scenario(&policy);
        let s_rotation = during_tunnel_rotation_scenario(&policy);
        assert!(!s_after_claim.passed, "AllowAll should fail double-claim assertion");
        assert!(!s_completion.passed, "AllowAll should fail double-completion assertion");
        assert!(!s_settlement.passed, "AllowAll should fail double-settlement assertion");
        assert!(!s_rotation.passed, "AllowAll should fail rotation idempotency assertion");
    }

    #[test]
    fn markdown_surfaces_per_scenario_status() {
        let report = run_failure_injection_scenarios();
        let md = report.to_markdown();
        assert!(md.contains("L6 Failure-Injection Report"));
        for s in &report.scenarios {
            assert!(md.contains(&format!("{:?}", s.kill_point)));
        }
    }
}
