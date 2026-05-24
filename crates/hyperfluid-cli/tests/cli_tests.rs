// CLI integration tests
//
// Tests verify the CLI produces correct output with a real node,
// or graceful error when no node is running.
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10

use std::fs;
use std::process::Command;

fn hyperfluid_binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_hyperfluid"));
    cmd.env("HYPERFLUID_NODE_URL", "http://127.0.0.1:19999");
    cmd
}

fn with_temp_ideas_dir(test_fn: impl FnOnce(&std::path::Path)) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp =
        std::env::temp_dir().join(format!("hyperfluid-ideas-test-{}-{}", std::process::id(), n));
    fs::create_dir_all(&tmp).unwrap();
    let seed_md = tmp.join("protocol-optimizations.md");
    fs::write(&seed_md, "\
## Title
Protocol Optimizations

## Short description
Improving the performance and efficiency of consensus protocols.

## Problem domain
Consensus protocols face bottlenecks as validator counts grow. Optimizations in block propagation, signature aggregation, and state sync reduce latency and bandwidth, making the network more scalable.

## Example tasks
- [Benchmark block propagation latency across 100 validators]
- [Implement BLS signature aggregation for validator votes]
- [Profile and optimise SMT root computation hot path]
- [Design compression scheme for state sync snapshots]

## Skills likely required
- [Rust systems programming]: core protocol is Rust
- [Distributed systems]: consensus protocol design
- [Cryptography]: BLS signatures, threshold schemes

## Tags
consensus, protocol, performance, cryptography
").unwrap();
    test_fn(&tmp);
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn cli_help_outputs_commands() {
    let output = hyperfluid_binary().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tx"));
    assert!(stdout.contains("query"));
    assert!(stdout.contains("task"));
    assert!(stdout.contains("review"));
    assert!(stdout.contains("governance"));
    assert!(stdout.contains("agent"));
    assert!(stdout.contains("idea"));
}

#[test]
fn cli_tx_transfer_reports_connection_error() {
    let output = hyperfluid_binary()
        .args([
            "tx",
            "transfer",
            "--sender",
            "01",
            "--recipient",
            "02",
            "--amount",
            "1000",
            "--nonce",
            "1",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "should exit with error when node unreachable");
    assert!(stderr.contains("Error"), "stderr should contain error: {}", stderr);
}

#[test]
fn cli_tx_bond_reports_connection_error() {
    let output = hyperfluid_binary()
        .args(["tx", "bond", "--validator", "03", "--amount", "5000", "--nonce", "1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn cli_query_balance_reports_connection_error() {
    let output = hyperfluid_binary()
        .args([
            "query",
            "balance",
            "--account",
            "aabbccdd11223344aabbccdd11223344aabbccdd11223344aabbccdd11223344",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn cli_idea_list_works_from_filesystem() {
    with_temp_ideas_dir(|dir| {
        let output = hyperfluid_binary()
            .env("HYPERFLUID_IDEAS_DIR", dir)
            .args(["idea", "list", "--output", "json"])
            .output()
            .unwrap();
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"idea_list\""));
        assert!(stdout.contains("\"protocol-optimizations\""));
        assert!(stdout.contains("\"Protocol Optimizations\""));
        assert!(stdout.contains("\"consensus\""));
        assert!(stdout.contains("\"count\": 1"));
    });
}

#[test]
fn cli_idea_show_reads_full_seed_file() {
    with_temp_ideas_dir(|dir| {
        let output = hyperfluid_binary()
            .env("HYPERFLUID_IDEAS_DIR", dir)
            .args(["idea", "show", "--slug", "protocol-optimizations", "--output", "json"])
            .output()
            .unwrap();
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"idea_show\""));
        assert!(stdout.contains("\"Protocol Optimizations\""));
        assert!(stdout.contains("Benchmark block propagation"));
        assert!(stdout.contains("\"consensus\""));
        assert!(stdout.contains("Improving the performance"));
    });
}

#[test]
fn cli_idea_show_reports_missing_slug() {
    with_temp_ideas_dir(|dir| {
        let output = hyperfluid_binary()
            .env("HYPERFLUID_IDEAS_DIR", dir)
            .args(["idea", "show", "--slug", "nonexistent"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not found"));
    });
}

#[test]
fn cli_agent_register_reports_not_implemented() {
    let output = hyperfluid_binary()
        .args(["agent", "register", "--name", "test-agent", "--tags", "rust,wasm"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not_implemented"));
    assert!(stdout.contains("Registration"));
}

#[test]
fn cli_help_lists_all_subcommands() {
    let output = hyperfluid_binary().arg("tx").arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("transfer"));
    assert!(stdout.contains("bond"));
    assert!(stdout.contains("delegate"));
}
