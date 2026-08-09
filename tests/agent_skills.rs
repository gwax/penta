//! The repository's agent instructions and skills have to reach every harness
//! that works on it, not just the one they were authored for. Codex discovers
//! `AGENTS.md` and `.agents/skills`; Claude Code discovers `CLAUDE.md` and
//! `.claude/skills`. Nothing in a normal build touches either set, so a moved
//! or renamed file would go unnoticed until an agent quietly lost a skill.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every skill the repository ships, by directory name.
const SKILLS: [&str; 3] = [
    "profile-engine-performance",
    "query-magic-references",
    "refresh-magic-references",
];

#[test]
fn claude_reads_the_same_repository_instructions_as_codex() {
    let root = repo_root();
    let claude = fs::read_to_string(root.join("CLAUDE.md")).expect("CLAUDE.md exists");

    assert!(
        root.join("AGENTS.md").is_file(),
        "AGENTS.md is the canonical instruction file"
    );
    assert!(
        claude.contains("@AGENTS.md"),
        "CLAUDE.md must import AGENTS.md so the two harnesses cannot drift; got:\n{claude}"
    );
}

#[test]
fn every_skill_is_discoverable_by_claude_and_by_codex() {
    let root = repo_root();

    for skill in SKILLS {
        let canonical = root.join(".agents/skills").join(skill).join("SKILL.md");
        assert!(
            canonical.is_file(),
            "{skill}: missing canonical .agents/skills/{skill}/SKILL.md"
        );

        let claude_copy = root.join(".claude/skills").join(skill).join("SKILL.md");
        assert!(
            claude_copy.is_file(),
            "{skill}: Claude Code only discovers .claude/skills/{skill}/SKILL.md; \
             add it (a symlink to the .agents copy) or Claude silently loses the skill"
        );

        // Symlinked rather than copied, so the two can never say different
        // things. Compare resolved paths instead of asserting on link metadata,
        // which keeps a real copy passing if a checkout cannot use symlinks.
        assert_eq!(
            fs::canonicalize(&claude_copy).expect("claude skill resolves"),
            fs::canonicalize(&canonical).expect("canonical skill resolves"),
            "{skill}: the Claude and Codex entry points must be the same file"
        );
    }
}

#[test]
fn skills_declare_a_name_matching_their_directory() {
    let root = repo_root();

    for skill in SKILLS {
        let path = root.join(".agents/skills").join(skill).join("SKILL.md");
        let text = fs::read_to_string(&path).expect("skill is readable");
        let declared = frontmatter_field(&text, "name")
            .unwrap_or_else(|| panic!("{skill}: SKILL.md has no `name:` in its frontmatter"));

        assert_eq!(
            declared, skill,
            "{skill}: the declared name must match the directory both harnesses key on"
        );
    }
}

#[test]
fn skill_descriptions_do_not_name_one_harness() {
    let root = repo_root();

    // A description is what a harness matches a task against. Naming a single
    // agent in it makes the skill read as not applying to any other one.
    for skill in SKILLS {
        let path = root.join(".agents/skills").join(skill).join("SKILL.md");
        let text = fs::read_to_string(&path).expect("skill is readable");
        let description = frontmatter_field(&text, "description")
            .unwrap_or_else(|| panic!("{skill}: SKILL.md has no `description:`"));
        let lowered = description.to_ascii_lowercase();

        for harness in ["codex", "claude", "copilot", "cursor"] {
            assert!(
                !lowered.contains(harness),
                "{skill}: description names {harness}; describe the work, not the agent"
            );
        }
    }
}

/// Reads one scalar out of the leading `---` frontmatter block, unwrapping the
/// surrounding quotes these files use for long values.
fn frontmatter_field(text: &str, field: &str) -> Option<String> {
    let body = text.strip_prefix("---\n")?;
    let end = body.find("\n---")?;
    let prefix = format!("{field}:");

    for line in body[..end].lines() {
        let Some(value) = line.strip_prefix(&prefix) else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(value);
        return Some(value.to_string());
    }
    None
}

/// Guards the one path shape that has to stay absolute-from-root: a skill body
/// is read through two different directories, so a link relative to the file
/// resolves to nothing under `.claude/skills`.
#[test]
fn skill_bodies_reference_repository_paths_from_the_root() {
    let root = repo_root();

    for skill in SKILLS {
        let path = root.join(".agents/skills").join(skill).join("SKILL.md");
        let text = fs::read_to_string(&path).expect("skill is readable");

        for (number, line) in text.lines().enumerate() {
            for target in markdown_link_targets(line) {
                if target.starts_with("http") || target.starts_with('#') {
                    continue;
                }
                assert!(
                    !target.starts_with("./") && !target.starts_with("../"),
                    "{skill}:{}: link {target:?} is relative to the file, which breaks \
                     when the skill is loaded through .claude/skills; write it from the \
                     repository root",
                    number + 1
                );
                assert!(
                    root.join(target).exists(),
                    "{skill}:{}: link {target:?} does not resolve from the repository root",
                    number + 1
                );
            }
        }
    }
}

fn markdown_link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;

    while let Some(open) = rest.find("](") {
        let after = &rest[open + 2..];
        let Some(close) = after.find(')') else { break };
        let target = &after[..close];
        if !target.is_empty() {
            targets.push(target);
        }
        rest = &after[close..];
    }
    targets
}

/// A skill that shells out to a script is only useful if the script is there.
#[test]
fn the_scripts_the_skills_invoke_exist() {
    for script in [
        ".agents/skills/profile-engine-performance/scripts/profile_attribution.py",
        ".agents/skills/refresh-magic-references/scripts/reference_material.py",
        "scripts/benchmark_engine.py",
    ] {
        let script = Path::new(script);
        assert!(
            repo_root().join(script).is_file(),
            "a skill invokes {}, by a path written from the repository root",
            script.display()
        );
    }
}
