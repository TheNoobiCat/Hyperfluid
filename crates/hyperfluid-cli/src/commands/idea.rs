use std::fs;
use std::path::PathBuf;

use clap::Subcommand;
use serde::Serialize;

use crate::commands::format_output;
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum IdeaAction {
    List {
        #[arg(long)]
        search: Option<String>,
        #[arg(long)]
        category: Option<String>,
    },
    Show {
        #[arg(long)]
        slug: String,
    },
}

#[derive(Serialize)]
struct SeedIdea {
    slug: String,
    title: String,
    description: String,
    problem_domain: String,
    example_tasks: Vec<String>,
    skills: Vec<String>,
    tags: Vec<String>,
}

pub fn run(
    action: IdeaAction,
    format: OutputFormat,
    _client: &reqwest::blocking::Client,
    _node_url: &str,
) -> Result<String, String> {
    let ideas_dir = resolve_ideas_dir()?;

    let output = match action {
        IdeaAction::List { search, category } => {
            let mut ideas = list_seeds(&ideas_dir)?;
            if let Some(q) = search {
                let q = q.to_lowercase();
                ideas.retain(|s| {
                    s.title.to_lowercase().contains(&q)
                        || s.description.to_lowercase().contains(&q)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                });
            }
            if let Some(cat) = category {
                let cat = cat.to_lowercase();
                ideas.retain(|s| s.tags.iter().any(|t| t.to_lowercase() == cat));
            }
            serde_json::json!({
                "action": "idea_list",
                "count": ideas.len(),
                "results": ideas.iter().map(|s| {
                    serde_json::json!({
                        "slug": s.slug,
                        "title": s.title,
                        "tags": s.tags,
                        "description": s.description,
                    })
                }).collect::<Vec<_>>(),
            })
        }
        IdeaAction::Show { slug } => {
            let file_path = ideas_dir.join(format!("{}.md", slug));
            if !file_path.exists() {
                return Err(format!("seed idea '{}' not found in {:?}", slug, ideas_dir));
            }
            let seed = parse_seed_file(&file_path, &slug)?;
            serde_json::json!({
                "action": "idea_show",
                "seed": seed,
            })
        }
    };

    Ok(format_output(&output, format))
}

fn resolve_ideas_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("HYPERFLUID_IDEAS_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(format!("HYPERFLUID_IDEAS_DIR set but '{}' is not a directory", dir));
    }
    let cwd = PathBuf::from("ideas");
    if cwd.is_dir() {
        return Ok(cwd);
    }
    Err("ideas directory not found — set HYPERFLUID_IDEAS_DIR or run from project root".into())
}

fn list_seeds(dir: &PathBuf) -> Result<Vec<SeedIdea>, String> {
    let mut seeds = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("cannot read ideas dir: {}", e))? {
        let entry = entry.map_err(|e| format!("dir entry error: {}", e))?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
        if name.is_empty() || name == "_template" || name == "README" {
            continue;
        }
        let seed = parse_seed_file(&path, name)?;
        seeds.push(seed);
    }
    seeds.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(seeds)
}

fn parse_seed_file(path: &PathBuf, slug: &str) -> Result<SeedIdea, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;

    let mut title = String::new();
    let mut description = String::new();
    let mut problem_domain = String::new();
    let mut example_tasks: Vec<String> = Vec::new();
    let mut skills: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();

    let mut current_section = "";
    let mut section_lines: Vec<&str> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = line.strip_prefix("## ") {
            if !section_lines.is_empty() {
                assign_section(
                    current_section,
                    &section_lines,
                    &mut title,
                    &mut description,
                    &mut problem_domain,
                    &mut example_tasks,
                    &mut skills,
                    &mut tags,
                );
            }
            current_section = header;
            section_lines = Vec::new();
        } else {
            section_lines.push(line);
        }
    }
    if !section_lines.is_empty() {
        assign_section(
            current_section,
            &section_lines,
            &mut title,
            &mut description,
            &mut problem_domain,
            &mut example_tasks,
            &mut skills,
            &mut tags,
        );
    }

    if title.is_empty() {
        return Err(format!("seed file {} is missing '## Title' section", path.display()));
    }

    Ok(SeedIdea {
        slug: slug.into(),
        title,
        description,
        problem_domain,
        example_tasks,
        skills,
        tags,
    })
}

#[allow(clippy::too_many_arguments)]
fn assign_section(
    header: &str,
    lines: &[&str],
    title: &mut String,
    description: &mut String,
    problem_domain: &mut String,
    example_tasks: &mut Vec<String>,
    skills: &mut Vec<String>,
    tags: &mut Vec<String>,
) {
    match header.to_lowercase().as_str() {
        "title" => {
            *title = lines.first().map(|s| s.to_string()).unwrap_or_default();
        }
        "short description" => {
            *description = lines.join(" ").chars().take(200).collect();
        }
        "problem domain" => {
            *problem_domain = lines.join("\n");
        }
        "example tasks" => {
            *example_tasks = lines
                .iter()
                .filter_map(|l| {
                    l.strip_prefix("- [")
                        .or_else(|| l.strip_prefix("- "))
                        .map(|s| s.trim().trim_end_matches(']').trim().to_string())
                })
                .filter(|l| !l.is_empty())
                .collect();
        }
        "skills likely required" | "skills" => {
            *skills = lines
                .iter()
                .map(|l| {
                    l.strip_prefix("- [").or_else(|| l.strip_prefix("- ")).unwrap_or(l).to_string()
                })
                .filter(|l| !l.is_empty())
                .collect();
        }
        "tags" => {
            *tags = lines
                .first()
                .map(|l| {
                    l.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
                })
                .unwrap_or_default();
        }
        unrecognized => {
            eprintln!("skipping unrecognized markdown section: {}", unrecognized);
        }
    }
}
