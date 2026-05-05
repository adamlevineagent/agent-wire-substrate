use agent_wire_substrate::{build_parity_demo, substrate_stack_name};
use agent_wire_substrate_node::{
    observe_l6_stability, run_d3_live_compute_settlement, run_l6_recovery_injection_scenarios,
    run_l6_stability_driver, run_layer3_single_graph_synthetic,
    run_layer4_two_graph_bridged_synthetic, run_layer5_live_llm_compute_roundtrip,
    run_live_contribution_sync, run_mainnet_auth, run_v1_node_cli,
};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match run_v1_node_cli(&args) {
        Ok(Some(output)) => {
            println!("{output}");
            return;
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("failed to run V1 node command: {error}");
            std::process::exit(1);
        }
    }

    let command = args.first().map(String::as_str);
    match command {
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
            let observability = observe_l6_stability(&report);
            print!("{}", observability.to_markdown());
            if !report.all_green() || !observability.all_invariants_held {
                std::process::exit(1);
            }
        }
        Some("l6-failure-injection") => {
            let report = run_l6_recovery_injection_scenarios();
            print!("{}", report.to_markdown());
            if !report.all_passed() {
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
            println!(
                "  surface                      Print the V1 CLI/MCP/HTTP/maintenance manifest"
            );
            println!("  identity signup|login|status Print typed identity protocol binding");
            println!("  identity persist [state-dir] Persist demo identity state atomically");
            println!("  identity load [state-dir]    Load persisted V1 identity state");
            println!("  chain compile <chain.yaml>   Compile a canonical Wire action chain");
            println!("  chain execute <chain.yaml>   Compile and locally route an action chain");
            println!("  chain quote <chain.yaml>     Produce a quote-mode compiled plan");
            println!("  compute offer|quote|purchase|fill|jobs");
            println!("                               Print typed compute protocol binding");
            println!("  mcp manifest                 Print V1 MCP tool bindings");
            println!(
                "  mcp dispatch <tool>          Dispatch one MCP request through typed registry"
            );
            println!("  http manifest                Print V1 HTTP route bindings");
            println!("  http dispatch <method> <path>");
            println!(
                "                               Dispatch one HTTP request through typed registry"
            );
            println!("  http smoke                   Run one loopback HTTP listener smoke");
            println!("  maintenance run-once         Fire local maintenance tasks and log stubs");
            println!("  maintenance schedule-tick    Run one typed maintenance scheduler tick");
            println!(
                "  runtime smoke [state-dir]    Smoke HTTP/MCP listeners, identity, scheduler"
            );
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
            println!("  l6-failure-injection         Run L6 recovery policy kill-point scenarios");
        }
        _ => println!("{}", substrate_stack_name()),
    }
}
