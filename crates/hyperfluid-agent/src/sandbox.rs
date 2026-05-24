// === C10 Agent Runtime: Review Sandbox Subagent ===
//
// Spawns a sandboxed child process to review artifacts and evidence.
// Enforces path-canonicalization to prevent file-system escapes.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Configuration for a sandbox review run.
pub struct SandboxConfig {
    /// Path to the artifact file to review.
    pub artifact_path: String,
    /// Path to the evidence file to review.
    pub evidence_path: String,
    /// Maximum time in seconds for the sandbox process to complete.
    pub timeout_secs: u64,
    /// Working directory for sandbox operations.
    pub working_dir: String,
}

/// Result of a sandbox review.
pub struct SandboxVerdict {
    /// The verdict decision.
    pub verdict: Verdict,
    /// Human-readable reason for the verdict.
    pub reason: String,
    /// SHA-256 hash of the evidence file.
    pub evidence_hash: [u8; 32],
}

/// Possible sandbox verdicts.
pub enum Verdict {
    /// Artifact accepted.
    Accept,
    /// Artifact rejected.
    Reject,
}

/// Run a sandbox review by spawning the agent binary as a child process.
///
/// 1. Creates a temporary directory inside `working_dir`.
/// 2. Copies the artifact and evidence files into the temp directory.
/// 3. Spawns the agent binary with `--sandbox-review` arguments.
/// 4. Waits for completion with a timeout.
/// 5. Parses the JSON verdict from stdout.
pub fn run_sandbox(config: &SandboxConfig) -> Result<SandboxVerdict, String> {
    // Validate that artifact and evidence paths exist
    let art_path = Path::new(&config.artifact_path);
    let ev_path = Path::new(&config.evidence_path);
    if !art_path.exists() {
        return Err(format!("Artifact path does not exist: {}", config.artifact_path));
    }
    if !ev_path.exists() {
        return Err(format!("Evidence path does not exist: {}", config.evidence_path));
    }

    // Create working directory if needed
    let work_base = Path::new(&config.working_dir);
    fs::create_dir_all(work_base).map_err(|e| format!("Failed to create working dir: {}", e))?;

    // Create a unique temp directory inside working_dir
    let temp_dir = work_base.join(format!("sandbox_{}", std::process::id()));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create sandbox temp dir: {}", e))?;

    // Canonicalize the temp dir for path guard
    let canonical_work = temp_dir
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize working dir: {}", e))?;

    // Copy artifact and evidence into the sandbox
    let dest_artifact = canonical_work
        .join(art_path.file_name().ok_or_else(|| "Invalid artifact path".to_string())?);
    let dest_evidence = canonical_work
        .join(ev_path.file_name().ok_or_else(|| "Invalid evidence path".to_string())?);

    fs::copy(art_path, &dest_artifact).map_err(|e| format!("Failed to copy artifact: {}", e))?;
    fs::copy(ev_path, &dest_evidence).map_err(|e| format!("Failed to copy evidence: {}", e))?;

    // Find the agent binary path (current executable)
    let agent_bin = std::env::current_exe()
        .map_err(|e| format!("Failed to determine agent binary path: {}", e))?;

    // Spawn the child process
    let mut child = Command::new(&agent_bin)
        .args([
            "--sandbox-review",
            &dest_artifact.to_string_lossy(),
            &dest_evidence.to_string_lossy(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&canonical_work)
        .spawn()
        .map_err(|e| format!("Failed to spawn sandbox process: {}", e))?;

    // Wait with timeout
    let deadline = std::time::Instant::now() + Duration::from_secs(config.timeout_secs);
    let output = loop {
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Sandbox process timed out".to_string());
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to collect sandbox output: {}", e))?;
                if !status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!(
                        "Sandbox process failed (code: {:?}): {}",
                        status.code(),
                        stderr
                    ));
                }
                break output;
            }
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Sandbox process error: {}", e));
            }
        }
    };

    // Parse JSON verdict from stdout
    let stdout_str =
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 in stdout: {}", e))?;
    let trimmed = stdout_str.trim();

    let json: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse sandbox JSON: {}", e))?;

    let verdict_str = json["verdict"]
        .as_str()
        .ok_or_else(|| "Missing 'verdict' field in sandbox output".to_string())?;
    let reason = json["reason"].as_str().unwrap_or("").to_string();
    let evidence_hash_hex = json["evidence_hash"]
        .as_str()
        .ok_or_else(|| "Missing 'evidence_hash' field in sandbox output".to_string())?;

    let verdict = match verdict_str.to_lowercase().as_str() {
        "accept" => Verdict::Accept,
        "reject" => Verdict::Reject,
        other => {
            return Err(format!("Unknown verdict '{}' — expected 'accept' or 'reject'", other))
        }
    };

    let mut evidence_hash = [0u8; 32];
    let decoded =
        hex::decode(evidence_hash_hex).map_err(|e| format!("Invalid evidence_hash hex: {}", e))?;
    if decoded.len() != 32 {
        return Err(format!("evidence_hash must be 32 bytes, got {}", decoded.len()));
    }
    evidence_hash.copy_from_slice(&decoded);

    Ok(SandboxVerdict { verdict, reason, evidence_hash })
}

/// Read a file with path-canonicalization guard.
///
/// Canonicalizes the requested `path` and verifies it is inside `working_dir`.
/// Returns the file contents or an error if the path is outside the sandbox.
pub fn sandbox_read_file(path: &str, working_dir: &str) -> Result<String, String> {
    let canonical_path = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path '{}': {}", path, e))?;
    let canonical_work = Path::new(working_dir)
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize working dir '{}': {}", working_dir, e))?;

    // Verify the path is a prefix of canonical_work
    if !canonical_path.starts_with(&canonical_work) {
        return Err(format!(
            "Path '{}' is outside sandbox working directory '{}'",
            canonical_path.display(),
            canonical_work.display()
        ));
    }

    fs::read_to_string(&canonical_path)
        .map_err(|e| format!("Failed to read file '{}': {}", canonical_path.display(), e))
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_sandbox_read_file_inside_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(b"hello sandbox").unwrap();
        f.flush().unwrap();

        let result = sandbox_read_file(file_path.to_str().unwrap(), dir.path().to_str().unwrap());
        assert!(result.is_ok(), "reading file inside working dir should succeed");
        assert_eq!(result.unwrap(), "hello sandbox");
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_sandbox_read_file_outside_dir() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file_path = outside.path().join("outside.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(b"outside").unwrap();
        f.flush().unwrap();

        let result = sandbox_read_file(file_path.to_str().unwrap(), dir.path().to_str().unwrap());
        assert!(result.is_err(), "reading file outside working dir should fail");
        assert!(
            result.unwrap_err().contains("outside sandbox"),
            "error should mention path restriction"
        );
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_sandbox_run_fails_with_missing_artifact() {
        let config = SandboxConfig {
            artifact_path: "/nonexistent/artifact.md".into(),
            evidence_path: "/nonexistent/evidence.md".into(),
            timeout_secs: 10,
            working_dir: std::env::temp_dir().to_string_lossy().to_string(),
        };
        let result = run_sandbox(&config);
        assert!(result.is_err(), "non-existent artifact should fail");
    }
}
