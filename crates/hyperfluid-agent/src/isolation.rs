// === C10 Agent Runtime: Process Isolation & Resource Limits ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 4
// Imports: crate::types::ResourceLimits

use crate::types::ResourceLimits;
use std::fs;
use std::path::{Path, PathBuf};

// ── Protocol Constants ──

pub const PROTOCOL_MIN_RAM: u64 = 512 * 1024 * 1024; // 512 MB
pub const PROTOCOL_MAX_RAM: u64 = 64 * 1024 * 1024 * 1024; // 64 GB
pub const PROTOCOL_MIN_DISK: u64 = 100 * 1024 * 1024; // 100 MB
pub const PROTOCOL_MAX_DISK: u64 = 1024u64 * 1024 * 1024 * 1024; // 1 TB

// ── SandboxConfig ──

pub struct SandboxConfig {
    pub limits: ResourceLimits,
    pub working_dir: PathBuf,
    pub node_api_url: String,
}

impl SandboxConfig {
    // ── Construction ──

    pub fn new(limits: ResourceLimits, working_dir: PathBuf, node_api_url: String) -> Self {
        Self { limits, working_dir, node_api_url }
    }

    // ── Validation ──

    /// Validate that all resource limits are within acceptable ranges.
    /// Returns Err with descriptive message if any limit is out of range.
    pub fn validate_limits(&self) -> Result<(), String> {
        let l = &self.limits;

        if l.max_ram_bytes < PROTOCOL_MIN_RAM {
            return Err(format!(
                "max_ram_bytes ({}) is below protocol minimum ({} bytes / 512 MB)",
                l.max_ram_bytes, PROTOCOL_MIN_RAM
            ));
        }
        if l.max_ram_bytes > PROTOCOL_MAX_RAM {
            return Err(format!(
                "max_ram_bytes ({}) exceeds protocol maximum ({} bytes / 64 GB)",
                l.max_ram_bytes, PROTOCOL_MAX_RAM
            ));
        }
        if l.max_cpu_cores < 1 {
            return Err(format!("max_cpu_cores ({}) is below minimum (1)", l.max_cpu_cores));
        }
        if l.max_cpu_cores > 64 {
            return Err(format!("max_cpu_cores ({}) exceeds maximum (64)", l.max_cpu_cores));
        }
        if l.cpu_throttle_pct < 10 {
            return Err(format!("cpu_throttle_pct ({}) is below minimum (10)", l.cpu_throttle_pct));
        }
        if l.cpu_throttle_pct > 100 {
            return Err(format!("cpu_throttle_pct ({}) exceeds maximum (100)", l.cpu_throttle_pct));
        }
        if l.max_disk_bytes < PROTOCOL_MIN_DISK {
            return Err(format!(
                "max_disk_bytes ({}) is below protocol minimum ({} bytes / 100 MB)",
                l.max_disk_bytes, PROTOCOL_MIN_DISK
            ));
        }
        if l.max_disk_bytes > PROTOCOL_MAX_DISK {
            return Err(format!(
                "max_disk_bytes ({}) exceeds protocol maximum ({} bytes / 1 TB)",
                l.max_disk_bytes, PROTOCOL_MAX_DISK
            ));
        }
        if l.max_file_descriptors < 64 {
            return Err(format!(
                "max_file_descriptors ({}) is below minimum (64)",
                l.max_file_descriptors
            ));
        }
        if l.max_file_descriptors > 65536 {
            return Err(format!(
                "max_file_descriptors ({}) exceeds maximum (65536)",
                l.max_file_descriptors
            ));
        }
        if l.max_concurrent_connections < 1 {
            return Err(format!(
                "max_concurrent_connections ({}) is below minimum (1)",
                l.max_concurrent_connections
            ));
        }
        if l.max_concurrent_connections > 10000 {
            return Err(format!(
                "max_concurrent_connections ({}) exceeds maximum (10000)",
                l.max_concurrent_connections
            ));
        }

        Ok(())
    }

    // ── Sandbox Boundary Checks ──

    /// Check whether `path` is within the sandbox working_dir.
    /// Uses canonicalize to resolve symlinks and check boundaries.
    /// Returns Err if path is outside working_dir or canonicalization fails.
    pub fn check_write_access(&self, path: &Path) -> Result<(), String> {
        if !self.is_within_sandbox(path) {
            return Err(format!(
                "Path '{}' is outside the sandbox working directory",
                path.display()
            ));
        }
        Ok(())
    }

    /// Returns true if `path` is within the sandbox working directory
    /// after canonicalization of both paths.
    pub fn is_within_sandbox(&self, path: &Path) -> bool {
        // Canonicalize the working directory.
        let canon_working = match self.working_dir.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        // Attempt to canonicalize the path. If it does not exist, walk up
        // to the nearest existing ancestor and canonicalize that, then
        // reconstruct the suffix to check containment.
        let canon_path = match canonicalize_allow_missing(path) {
            Some(p) => p,
            None => return false,
        };

        // A path is within the sandbox if the canonicalized working_dir is
        // a prefix of the canonicalized target path.
        canon_path.starts_with(&canon_working)
    }
}

// ── Disk Quota ──

/// Walk the working directory tree and sum file sizes.
/// Returns the total bytes used. Does not enforce — just reports.
/// Enforcement is the caller's responsibility.
pub fn enforce_disk_quota(working_dir: &Path, _max_bytes: u64) -> Result<u64, std::io::Error> {
    walk_dir_size(working_dir)
}

fn walk_dir_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut total: u64 = 0;
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            // If the directory does not exist, usage is 0.
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(0);
            }
            return Err(e);
        }
    };

    for entry in entries {
        let entry = entry?;
        let ft = entry.file_type()?;

        if ft.is_symlink() {
            // Skip symlinks to avoid double-counting and infinite loops.
            continue;
        }

        if ft.is_dir() {
            total += walk_dir_size(&entry.path())?;
        } else if ft.is_file() {
            total += entry.metadata()?.len();
        }
    }

    Ok(total)
}

// ── File Descriptor Limit Check ──

/// Best-effort check of the current file descriptor limit.
///
/// On Linux: reads `/proc/self/limits` and parses the soft limit for
/// "Max open files". Returns the parsed value on success.
///
/// On all other platforms (including Windows): returns `Ok(0)` to
/// indicate the check is not available.
///
/// Actual enforcement is via OS cgroups/limits; this is informational.
pub fn check_file_descriptor_limit(_max_fds: u32) -> Result<u32, String> {
    #[cfg(target_os = "linux")]
    {
        let content = match std::fs::read_to_string("/proc/self/limits") {
            Ok(c) => c,
            Err(e) => {
                // File not available (e.g. inside a restricted container).
                tracing::warn!("Cannot read /proc/self/limits (fd limit check unavailable): {}", e);
                return Ok(0);
            }
        };

        for line in content.lines() {
            if line.starts_with("Max open files") {
                // Format: "Max open files            1024                 4096                 files"
                // Soft limit is the second field after the label.
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Ok(soft_limit) = parts[3].parse::<u32>() {
                        return Ok(soft_limit);
                    }
                }
                break;
            }
        }

        // Parsed but couldn't extract the value.
        Ok(0)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = _max_fds;
        Ok(0)
    }
}

// ── Violation Logging ──

/// Log a sandbox violation at error level.
pub fn log_sandbox_violation(violation: &str) {
    tracing::error!(
        target: "hyperfluid_agent::isolation",
        violation = %violation,
        "SANDBOX VIOLATION: {}",
        violation,
    );
}

// ── Helpers ──

/// Attempt to canonicalize a path that may not exist by walking up to
/// the nearest existing ancestor, canonicalizing that, then appending
/// the missing suffix.
fn canonicalize_allow_missing(path: &Path) -> Option<PathBuf> {
    if let Ok(canon) = path.canonicalize() {
        return Some(canon);
    }

    // Walk up to find an existing ancestor.
    let mut existing = path.to_path_buf();
    let mut suffix = PathBuf::new();

    loop {
        if existing.canonicalize().is_ok() {
            let canon = existing.canonicalize().ok()?;
            return Some(if suffix.as_os_str().is_empty() { canon } else { canon.join(&suffix) });
        }

        let component = existing.file_name()?.to_os_string();
        suffix = PathBuf::from(component).join(&suffix);

        if !existing.pop() {
            // Reached root without finding anything that exists.
            return None;
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn default_limits() -> ResourceLimits {
        ResourceLimits::default()
    }

    // ── Test 1: valid_limits_pass_validation ──

    #[test]
    fn valid_limits_pass_validation() {
        let cfg = SandboxConfig::new(
            default_limits(),
            PathBuf::from("/tmp/hyperfluid-sandbox"),
            "http://127.0.0.1:8080".into(),
        );
        assert!(cfg.validate_limits().is_ok());
    }

    // ── Test 2: ram_below_minimum_fails ──

    #[test]
    fn ram_below_minimum_fails() {
        let mut limits = default_limits();
        limits.max_ram_bytes = 100 * 1024 * 1024; // 100 MB
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_ram_bytes"));
    }

    // ── Test 3: ram_above_maximum_fails ──

    #[test]
    fn ram_above_maximum_fails() {
        let mut limits = default_limits();
        limits.max_ram_bytes = 128 * 1024 * 1024 * 1024u64; // 128 GB
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_ram_bytes"));
    }

    // ── Test 4: cpu_cores_too_low_fails ──

    #[test]
    fn cpu_cores_too_low_fails() {
        let mut limits = default_limits();
        limits.max_cpu_cores = 0;
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_cpu_cores"));
    }

    // ── Test 5: disk_too_small_fails ──

    #[test]
    fn disk_too_small_fails() {
        let mut limits = default_limits();
        limits.max_disk_bytes = 1024 * 1024; // 1 MB
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_disk_bytes"));
    }

    // ── Test 6: fd_limit_too_low_fails ──

    #[test]
    fn fd_limit_too_low_fails() {
        let mut limits = default_limits();
        limits.max_file_descriptors = 32;
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_file_descriptors"));
    }

    // ── Test 7: fd_limit_too_high_fails ──

    #[test]
    fn fd_limit_too_high_fails() {
        let mut limits = default_limits();
        limits.max_file_descriptors = 100000;
        let cfg = SandboxConfig::new(limits, PathBuf::from("/tmp"), String::new());
        let result = cfg.validate_limits();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_file_descriptors"));
    }

    // ── Test 8: path_within_sandbox_passes ──

    #[test]
    fn path_within_sandbox_passes() {
        let tmp = std::env::temp_dir().join("hyperfluid-test-sandbox");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let cfg = SandboxConfig::new(default_limits(), tmp.clone(), String::new());

        // Create a file inside the sandbox.
        let inner_file = tmp.join("allowed.txt");
        fs::write(&inner_file, b"hello").unwrap();

        assert!(cfg.is_within_sandbox(&inner_file));
        assert!(cfg.check_write_access(&inner_file).is_ok());

        // Cleanup.
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Test 9: path_outside_sandbox_fails ──

    #[test]
    fn path_outside_sandbox_fails() {
        let tmp = std::env::temp_dir().join("hyperfluid-test-sandbox-9");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let cfg = SandboxConfig::new(default_limits(), tmp.clone(), String::new());

        // This path is outside the sandbox.
        let outside = std::env::temp_dir().join("outside-file.txt");
        fs::write(&outside, b"should be blocked").unwrap();

        assert!(!cfg.is_within_sandbox(&outside));
        assert!(cfg.check_write_access(&outside).is_err());

        // Cleanup.
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Test 10: path_with_dot_dot_blocked ──

    #[test]
    fn path_with_dot_dot_blocked() {
        let tmp = std::env::temp_dir().join("hyperfluid-test-sandbox-10");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let subdir = tmp.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let cfg = SandboxConfig::new(default_limits(), subdir.clone(), String::new());

        // Try to escape via ../  pointing to a sibling file.
        let sibling = tmp.join("outside.txt");
        fs::write(&sibling, b"escape attempt").unwrap();

        let escape_attempt = subdir.join("..").join("outside.txt");
        // Normalize to avoid test-framework collapsing it early.
        let normalized = escape_attempt.components().collect::<PathBuf>();

        // Even if the path resolves to outside, the sandbox should block it.
        // Note: on some platforms, canonicalize resolves ../ away, so the
        // real assertion is that the resolved path is not prefixed by the
        // canonicalized working_dir.
        assert!(!cfg.is_within_sandbox(&normalized));

        // Cleanup.
        let _ = fs::remove_file(&sibling);
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Test 11: disk_quota_reports_usage ──

    #[test]
    fn disk_quota_reports_usage() {
        let tmp = std::env::temp_dir().join("hyperfluid-test-quota");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Create a few files with known sizes.
        let file_a = tmp.join("a.txt");
        let file_b = tmp.join("b.txt");
        let sub = tmp.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file_c = sub.join("c.txt");

        {
            let mut f = fs::File::create(&file_a).unwrap();
            f.write_all(&[0u8; 100]).unwrap();
        }
        {
            let mut f = fs::File::create(&file_b).unwrap();
            f.write_all(&[0u8; 200]).unwrap();
        }
        {
            let mut f = fs::File::create(&file_c).unwrap();
            f.write_all(&[0u8; 50]).unwrap();
        }

        let total = enforce_disk_quota(&tmp, 1_000_000).unwrap();
        // 100 + 200 + 50 = 350 (allow for directory metadata rounding).
        assert!(total >= 350, "expected at least 350 bytes, got {}", total);
        assert!(total <= 360, "expected at most 360 bytes, got {}", total);

        // Cleanup.
        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Test 12: sandbox_config_construction ──

    #[test]
    fn sandbox_config_construction() {
        let limits = default_limits();
        let dir = PathBuf::from("/tmp/hyperfluid-agent");
        let url = "http://127.0.0.1:9090".to_string();

        let cfg = SandboxConfig::new(limits.clone(), dir.clone(), url.clone());

        assert_eq!(cfg.limits.max_ram_bytes, limits.max_ram_bytes);
        assert_eq!(cfg.limits.max_cpu_cores, limits.max_cpu_cores);
        assert_eq!(cfg.limits.cpu_throttle_pct, limits.cpu_throttle_pct);
        assert_eq!(cfg.limits.max_disk_bytes, limits.max_disk_bytes);
        assert_eq!(cfg.limits.max_file_descriptors, limits.max_file_descriptors);
        assert_eq!(cfg.limits.max_concurrent_connections, limits.max_concurrent_connections);
        assert_eq!(cfg.working_dir, dir);
        assert_eq!(cfg.node_api_url, url);
    }
}
