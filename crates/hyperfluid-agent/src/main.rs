fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--setup" {
        hyperfluid_agent::tui::run_setup();
    } else {
        eprintln!("Usage: hyperfluid-agent --setup");
        eprintln!("  --setup  Launch interactive TUI configuration wizard");
        std::process::exit(1);
    }
}
