use agent_wire_substrate_node::{
    build_parity_demo, run_d3_live_compute_settlement, run_l6_stability_driver,
    run_layer3_single_graph_synthetic, run_layer4_two_graph_bridged_synthetic,
    run_layer5_live_llm_compute_roundtrip, run_live_contribution_sync, run_mainnet_auth,
    substrate_stack_name,
};

fn main() {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("substrate-node-demo") => match build_parity_demo() {
            Ok(report) => print!("{}", report.to_markdown()),
            Err(error) => {
                eprintln!("failed to build substrate node demo: {error}");
                std::process::exit(1);
            }
        },
        Some("layer3-synthetic") => match run_layer3_single_graph_synthetic() {
            Ok(report) => print!("{}", report.to_markdown()),
            Err(error) => {
                eprintln!("failed to run Layer 3 synthetic validation: {error}");
                std::process::exit(1);
            }
        },
        Some("layer4-synthetic") => match run_layer4_two_graph_bridged_synthetic() {
            Ok(report) => print!("{}", report.to_markdown()),
            Err(error) => {
                eprintln!("failed to run Layer 4 synthetic validation: {error}");
                std::process::exit(1);
            }
        },
        Some("layer5-live-llm") => {
            let report = run_layer5_live_llm_compute_roundtrip();
            print!("{}", report.to_markdown());
            if !report.all_green() {
                std::process::exit(1);
            }
        }
        Some("d3-live-compute-settlement") => {
            let report = run_d3_live_compute_settlement();
            print!("{}", report.to_markdown());
            if !report.all_green() {
                std::process::exit(1);
            }
        }
        Some("l6-stability-driver") => {
            let report = run_l6_stability_driver();
            print!("{}", report.to_markdown());
            if !report.all_green() {
                std::process::exit(1);
            }
        }
        Some("auth") => {
            let report = run_mainnet_auth();
            print!("{}", report.to_markdown());
            if !report.all_green() {
                std::process::exit(1);
            }
        }
        Some("contribution-sync") => {
            let report = run_live_contribution_sync();
            print!("{}", report.to_markdown());
            if !report.all_green() {
                std::process::exit(1);
            }
        }
        Some("--help") | Some("-h") => {
            println!("agent-wire-substrate-node");
            println!();
            println!("Commands:");
            println!("  substrate-node-demo         Run the substrate-tier dry-run parity demo");
            println!("  auth                         Validate and persist mainnet auth state");
            println!(
                "  contribution-sync            Publish and read back a live Wire contribution"
            );
            println!("  layer3-synthetic             Run Wave 2 Layer 3 single-graph synthetic validation");
            println!(
                "  layer4-synthetic             Run Wave 2 Layer 4 two-graph bridged validation"
            );
            println!(
                "  layer5-live-llm              Run Wave 2 Layer 5 live LLM compute roundtrip"
            );
            println!(
                "  d3-live-compute-settlement   Run D3 live mainnet compute settlement validation"
            );
            println!(
                "  l6-stability-driver          Run repeated D3 cycles for L6 stability validation"
            );
        }
        _ => println!("{}", substrate_stack_name()),
    }
}
