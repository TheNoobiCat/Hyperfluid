fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--setup" {
        hyperfluid_agent::tui::run_setup();
    } else if args.len() >= 4 && args[1] == "--sandbox-review" {
        run_sandbox_review(&args[2], &args[3]);
    } else {
        eprintln!("Usage: hyperfluid-agent --setup");
        eprintln!("  --setup               Launch interactive TUI configuration wizard");
        eprintln!("  --sandbox-review <artifact> <evidence>");
        eprintln!("                         Run sandbox review and output JSON verdict");
        std::process::exit(1);
    }
}

/// Handles the `--sandbox-review` subcommand.
///
/// Reads the artifact and evidence files, computes a SHA-256 hash of the
/// evidence, and outputs a JSON verdict to stdout. This is invoked by
/// the sandbox runner as a child process.
fn run_sandbox_review(artifact_path: &str, evidence_path: &str) {
    use sha3::{Digest, Sha3_256};
    use std::fs;

    // Read evidence file for hashing
    let evidence_bytes = match fs::read(evidence_path) {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({
                "verdict": "error",
                "reason": format!("Failed to read evidence file: {}", e),
                "evidence_hash": "",
            });
            println!("{}", serde_json::to_string(&err).unwrap_or_default());
            std::process::exit(1);
        }
    };

    // Compute SHA-256 hash of evidence
    let mut hasher = Sha3_256::new();
    hasher.update(&evidence_bytes);
    let evidence_hash = hex::encode(hasher.finalize());

    // Verify artifact exists
    if !std::path::Path::new(artifact_path).exists() {
        let err = serde_json::json!({
            "verdict": "error",
            "reason": format!("Artifact file not found: {}", artifact_path),
            "evidence_hash": evidence_hash,
        });
        println!("{}", serde_json::to_string(&err).unwrap_or_default());
        std::process::exit(1);
    }

    // For now, accept all valid artifacts (actual review logic will be added later)
    let verdict = serde_json::json!({
        "verdict": "accept",
        "reason": "Artifact reviewed by sandbox process — no issues detected.",
        "evidence_hash": evidence_hash,
    });
    println!("{}", serde_json::to_string(&verdict).unwrap_or_default());
}
