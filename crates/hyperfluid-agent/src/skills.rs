// === C10 Agent Runtime: Skills Infrastructure ===
//
// Discovers, parses, and injects skill definitions from a skills directory.
// Each skill is defined by a SKILL.md file in a subdirectory of the skills base.

use std::fs;
use std::path::Path;

/// A skill definition loaded from a SKILL.md file.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Short name (directory name, used for referencing).
    pub name: String,
    /// Title from the first `# Title` line.
    pub title: String,
    /// First paragraph of description (after the title).
    pub description: String,
    /// Full instructions (everything after the description paragraph).
    pub instructions: String,
}

/// Scan a skills directory and return all discovered skills.
///
/// Expects the structure:
/// ```text
/// {skills_base}/
///   skill_name_a/
///     SKILL.md
///   skill_name_b/
///     SKILL.md
/// ```
///
/// Returns a vector of `(name, Skill)` pairs, where `name` is the directory name.
pub fn scan_skills_dir(skills_base: &str) -> Result<Vec<(String, Skill)>, String> {
    let base = Path::new(skills_base);
    if !base.is_dir() {
        return Err(format!("Skills directory does not exist: {}", skills_base));
    }

    let mut skills = Vec::new();

    let entries = fs::read_dir(base).map_err(|e| format!("Failed to read skills dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let dir_path = entry.path();

        if !dir_path.is_dir() {
            continue;
        }

        let skill_md_path = dir_path.join("SKILL.md");
        if !skill_md_path.is_file() {
            continue;
        }

        let name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Invalid directory name: {:?}", dir_path))?
            .to_string();

        let content = fs::read_to_string(&skill_md_path)
            .map_err(|e| format!("Failed to read SKILL.md: {}", e))?;

        let skill = parse_skill_md(&name, &content)?;
        skills.push((name, skill));
    }

    // Sort by name for deterministic order
    skills.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(skills)
}

/// Load a single skill by name from the skills directory.
pub fn load_skill_content(skills_base: &str, skill_name: &str) -> Result<Skill, String> {
    let skill_path = Path::new(skills_base).join(skill_name).join("SKILL.md");

    if !skill_path.is_file() {
        return Err(format!("Skill '{}' not found at {}", skill_name, skill_path.display()));
    }

    let content =
        fs::read_to_string(&skill_path).map_err(|e| format!("Failed to read SKILL.md: {}", e))?;
    parse_skill_md(skill_name, &content)
}

/// Parse a SKILL.md file into a `Skill` struct.
///
/// Format:
/// - First `# Title` line → `title`
/// - Next paragraph (text between title and next blank line or heading) → `description`
/// - Everything after → `instructions`
fn parse_skill_md(name: &str, content: &str) -> Result<Skill, String> {
    let mut lines = content.lines();

    // Find the first `# Title` line
    let title_line = lines
        .by_ref()
        .find(|l| l.starts_with("# "))
        .ok_or_else(|| format!("SKILL.md for '{}' has no '# Title' line", name))?;
    let title = title_line.trim_start_matches("# ").trim().to_string();

    // Skip blank lines after title
    let desc_lines: Vec<&str> = lines
        .by_ref()
        .skip_while(|l| l.trim().is_empty())
        .take_while(|l| !l.trim().is_empty() && !l.starts_with("# "))
        .collect();
    let description = desc_lines.join(" ").trim().to_string();

    // Everything remaining is instructions
    let remaining: Vec<&str> = lines.collect();
    let instructions = remaining.join("\n").trim().to_string();

    Ok(Skill { name: name.to_string(), title, description, instructions })
}

/// Append a "## Available Skills" section to the base prompt.
///
/// Each skill is listed as:
/// ```text
/// - **{name}**: {title} — {description}
/// ```
pub fn inject_skills_prompt(skills: &[(String, Skill)], base_prompt: &str) -> String {
    if skills.is_empty() {
        return base_prompt.to_string();
    }

    let mut output = String::with_capacity(base_prompt.len() + skills.len() * 120);
    output.push_str(base_prompt);
    output.push_str("\n\n## Available Skills\n\n");

    for (name, skill) in skills {
        output.push_str(&format!("- **{}**: {} — {}\n", name, skill.title, skill.description));
    }

    output
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_scan_and_parse_skills() {
        let dir = tempfile::tempdir().unwrap();

        // Create skill directory structure
        let code_dir = dir.path().join("code-review");
        fs::create_dir_all(&code_dir).unwrap();
        let mut f = fs::File::create(code_dir.join("SKILL.md")).unwrap();
        f.write_all(b"# Code Review\n\nAnalyze code for bugs and style issues.\nCheck formatting, logic, and edge cases.\n\nUse the following checklist:\n1. Check for errors\n2. Verify logic\n3. Review style\n")
            .unwrap();
        f.flush().unwrap();

        let docs_dir = dir.path().join("documentation");
        fs::create_dir_all(&docs_dir).unwrap();
        let mut f = fs::File::create(docs_dir.join("SKILL.md")).unwrap();
        f.write_all(b"# Documentation\n\nWrite clear documentation.\n\nFollow the style guide.\n")
            .unwrap();
        f.flush().unwrap();

        let skills = scan_skills_dir(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(skills.len(), 2);

        // Check code-review skill
        let (name, skill) = &skills[0];
        assert_eq!(name, "code-review");
        assert_eq!(skill.title, "Code Review");
        assert!(skill.description.contains("Analyze code"));
        assert!(skill.instructions.contains("checklist"));

        // Check documentation skill
        let (name2, skill2) = &skills[1];
        assert_eq!(name2, "documentation");
        assert_eq!(skill2.title, "Documentation");
        assert!(skill2.description.contains("Write clear documentation"));
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_load_single_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        f.write_all(b"# Test Skill\n\nA test skill description.\n\nInstruction body here.\n")
            .unwrap();
        f.flush().unwrap();

        let skill = load_skill_content(dir.path().to_str().unwrap(), "test-skill").unwrap();
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.title, "Test Skill");
        assert_eq!(skill.description, "A test skill description.");
        assert!(skill.instructions.contains("Instruction body"));
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_skills_prompt_injection() {
        let skill = Skill {
            name: "rust-review".into(),
            title: "Rust Review".into(),
            description: "Review Rust code for safety and correctness.".into(),
            instructions: "Check for unsafe blocks, ownership, etc.".into(),
        };
        let skills = vec![("rust-review".to_string(), skill)];
        let base = "You are a helpful assistant.";
        let result = inject_skills_prompt(&skills, base);

        assert!(result.contains("## Available Skills"));
        assert!(result.contains("**rust-review**"));
        assert!(result.contains("Rust Review"));
        assert!(result.contains("Review Rust code"));
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_empty_skills_no_injection() {
        let skills: Vec<(String, Skill)> = Vec::new();
        let base = "You are a helpful assistant.";
        let result = inject_skills_prompt(&skills, base);
        assert_eq!(result, base);
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_missing_skills_directory() {
        let result = scan_skills_dir("/nonexistent/skills/path");
        assert!(result.is_err());
    }

    #[test]
    fn conforms_to_agent_runtime_spec_sectionX_missing_skill_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_skill_content(dir.path().to_str().unwrap(), "nonexistent");
        assert!(result.is_err());
    }
}
