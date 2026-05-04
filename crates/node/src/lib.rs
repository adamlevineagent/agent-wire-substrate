//! Operator bundle composition crate.
//!
//! The boot/API split is intentionally outside foundation. This crate will host
//! substrate boot, vertical boot, substrate API, and vertical API composition.

pub fn substrate_stack_name() -> &'static str {
    "agent-wire-substrate"
}
