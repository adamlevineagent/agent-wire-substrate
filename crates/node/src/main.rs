use agent_wire_node::{build_parity_demo, substrate_stack_name};

fn main() {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("parity-demo") => match build_parity_demo() {
            Ok(report) => print!("{}", report.to_markdown()),
            Err(error) => {
                eprintln!("failed to build parity demo: {error}");
                std::process::exit(1);
            }
        },
        Some("--help") | Some("-h") => {
            println!("agent-wire-node");
            println!();
            println!("Commands:");
            println!("  parity-demo    Run the substrate-tier dry-run parity demo");
        }
        _ => println!("{}", substrate_stack_name()),
    }
}
