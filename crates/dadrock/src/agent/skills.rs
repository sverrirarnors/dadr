use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    error::DadrError,
    paths::{expand_tilde, home_dir},
};

pub const AGENT_PROMPT_PACK_BYTE_LIMIT: usize = 96 * 1024;

const BUILTIN_PROMPT_PACKS: &[(&str, &str, &str)] = &[];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPromptPack {
    pub name: String,
    pub source: Utf8PathBuf,
    pub sha256: String,
    pub instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPromptPackInfo {
    pub name: String,
    pub source: Utf8PathBuf,
    pub sha256: String,
}

impl From<&AgentPromptPack> for AgentPromptPackInfo {
    fn from(pack: &AgentPromptPack) -> Self {
        Self {
            name: pack.name.clone(),
            source: pack.source.clone(),
            sha256: pack.sha256.clone(),
        }
    }
}

pub fn resolve_prompt_packs(
    specs: &[String],
    project_cwd: &Utf8Path,
) -> Result<Vec<AgentPromptPack>, DadrError> {
    let project_search_dirs = project_prompt_pack_search_dirs(project_cwd);
    let user_search_dirs = user_prompt_pack_search_dirs();
    specs
        .iter()
        .map(|spec| resolve_prompt_pack(spec, project_cwd, &project_search_dirs, &user_search_dirs))
        .collect()
}

pub fn list_prompt_packs(project_cwd: &Utf8Path) -> Result<Vec<AgentPromptPackInfo>, DadrError> {
    let mut packs = BTreeMap::<(String, Utf8PathBuf), AgentPromptPackInfo>::new();
    for dir in project_prompt_pack_search_dirs(project_cwd) {
        collect_prompt_packs_from_dir(&mut packs, &dir)?;
    }
    for &(name, source, instructions) in BUILTIN_PROMPT_PACKS {
        let pack = prompt_pack_from_instructions(
            name.to_owned(),
            Utf8PathBuf::from(source),
            instructions.to_owned(),
        )?;
        let info = AgentPromptPackInfo::from(&pack);
        packs.insert((info.name.clone(), info.source.clone()), info);
    }
    for dir in user_prompt_pack_search_dirs() {
        collect_prompt_packs_from_dir(&mut packs, &dir)?;
    }
    Ok(packs.into_values().collect())
}

fn collect_prompt_packs_from_dir(
    packs: &mut BTreeMap<(String, Utf8PathBuf), AgentPromptPackInfo>,
    dir: &Path,
) -> Result<(), DadrError> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let Some(file) = listed_prompt_pack_file(&path) else {
            continue;
        };
        let name = prompt_pack_name_from_file(&file);
        let pack = read_prompt_pack(&file, name)?;
        let info = AgentPromptPackInfo::from(&pack);
        packs.insert((info.name.clone(), info.source.clone()), info);
    }
    Ok(())
}

pub fn combined_prompt_pack_sha256(packs: &[AgentPromptPack]) -> Option<String> {
    if packs.is_empty() {
        return None;
    }
    if packs.len() == 1 {
        return Some(packs[0].sha256.clone());
    }

    let mut hasher = Sha256::new();
    for pack in packs {
        hasher.update(pack.name.as_bytes());
        hasher.update(b"\0");
        hasher.update(pack.source.as_str().as_bytes());
        hasher.update(b"\0");
        hasher.update(pack.sha256.as_bytes());
        hasher.update(b"\0");
    }
    Some(hex_digest(hasher.finalize().as_slice()))
}

pub fn render_prompt_packs(packs: &[AgentPromptPack]) -> String {
    if packs.is_empty() {
        return String::new();
    }

    let mut rendered = String::from(
        "Dadrock prompt pack instructions follow. Treat these as task-specific operating instructions, while still obeying the Dadrock response contract.\n",
    );
    for pack in packs {
        rendered.push_str("\n--- Prompt pack: ");
        rendered.push_str(&pack.name);
        rendered.push_str(" (sha256: ");
        rendered.push_str(&pack.sha256);
        rendered.push_str(") ---\n");
        rendered.push_str(pack.instructions.trim());
        rendered.push('\n');
    }
    rendered
}

fn resolve_prompt_pack(
    spec: &str,
    project_cwd: &Utf8Path,
    project_search_dirs: &[PathBuf],
    user_search_dirs: &[PathBuf],
) -> Result<AgentPromptPack, DadrError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(DadrError::InvalidInput(
            "agent skill or prompt pack name cannot be empty".to_owned(),
        ));
    }

    if let Some(path) = explicit_prompt_pack_path(spec, project_cwd) {
        return read_prompt_pack(&path, prompt_pack_name(spec, &path));
    }

    for dir in project_search_dirs {
        let candidate = dir.join(spec);
        if let Some(skill_file) = prompt_pack_file(&candidate) {
            return read_prompt_pack(&skill_file, spec.to_owned());
        }
    }

    if let Some(pack) = builtin_prompt_pack(spec)? {
        return Ok(pack);
    }

    for dir in user_search_dirs {
        let candidate = dir.join(spec);
        if let Some(skill_file) = prompt_pack_file(&candidate) {
            return read_prompt_pack(&skill_file, spec.to_owned());
        }
    }

    Err(DadrError::InvalidInput(format!(
        "prompt pack or skill `{spec}` was not found; pass a path or install it under .agents/skills, .github/skills, .claude/skills, ~/.agents/skills, ~/.codex/skills, ~/.claude/skills, or $CODEX_HOME/skills"
    )))
}

fn builtin_prompt_pack(spec: &str) -> Result<Option<AgentPromptPack>, DadrError> {
    let Some(&(name, source, instructions)) = BUILTIN_PROMPT_PACKS
        .iter()
        .find(|(name, source, _instructions)| spec == *name || spec == *source)
    else {
        return Ok(None);
    };
    Ok(Some(prompt_pack_from_instructions(
        name.to_owned(),
        Utf8PathBuf::from(source),
        instructions.to_owned(),
    )?))
}

fn explicit_prompt_pack_path(spec: &str, project_cwd: &Utf8Path) -> Option<PathBuf> {
    let raw = expand_tilde(spec);
    let path = if raw.is_absolute() {
        raw
    } else {
        project_cwd.as_std_path().join(raw)
    };
    if path.exists() {
        prompt_pack_file(&path)
    } else if spec.contains('/') || spec.contains('\\') || spec.ends_with(".md") {
        Some(path)
    } else {
        None
    }
}

fn prompt_pack_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if path.is_dir() {
        for name in ["SKILL.md", "skill.md", "PROMPT_PACK.md", "prompt-pack.md"] {
            let candidate = path.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn listed_prompt_pack_file(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        return prompt_pack_file(path);
    }
    if path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Some(path.to_path_buf());
    }
    None
}

fn read_prompt_pack(path: &Path, name: String) -> Result<AgentPromptPack, DadrError> {
    let instructions = fs::read_to_string(path)?;
    prompt_pack_from_instructions(name, path_to_utf8(path.to_path_buf())?, instructions)
}

fn prompt_pack_from_instructions(
    name: String,
    source: Utf8PathBuf,
    instructions: String,
) -> Result<AgentPromptPack, DadrError> {
    let bytes = instructions.len();
    if bytes > AGENT_PROMPT_PACK_BYTE_LIMIT {
        return Err(DadrError::InvalidInput(format!(
            "prompt pack `{}` is {bytes} bytes, exceeding the prompt pack limit of {AGENT_PROMPT_PACK_BYTE_LIMIT} bytes",
            source
        )));
    }
    if instructions.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "prompt pack `{}` is empty",
            source
        )));
    }

    Ok(AgentPromptPack {
        name,
        source,
        sha256: sha256_hex(instructions.as_bytes()),
        instructions,
    })
}

fn prompt_pack_name(spec: &str, path: &Path) -> String {
    if !(spec.trim().is_empty() || spec.contains('/') || spec.contains('\\')) {
        return spec.to_owned();
    }
    prompt_pack_name_from_file(path)
}

fn prompt_pack_name_from_file(path: &Path) -> String {
    let is_pack_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "SKILL.md" | "skill.md" | "PROMPT_PACK.md" | "prompt-pack.md"
            )
        });
    if !is_pack_file {
        return path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("prompt-pack")
            .to_owned();
    }
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("prompt-pack")
        .to_owned()
}

#[cfg(test)]
fn prompt_pack_search_dirs(project_cwd: &Utf8Path) -> Vec<PathBuf> {
    let mut dirs = project_prompt_pack_search_dirs(project_cwd);
    dirs.extend(user_prompt_pack_search_dirs());
    dirs
}

fn project_prompt_pack_search_dirs(project_cwd: &Utf8Path) -> Vec<PathBuf> {
    vec![
        project_cwd.join(".agents/skills").into_std_path_buf(),
        project_cwd.join(".github/skills").into_std_path_buf(),
        project_cwd.join(".claude/skills").into_std_path_buf(),
    ]
}

fn user_prompt_pack_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home_dir() {
        dirs.push(home.join(".agents/skills"));
        dirs.push(home.join(".codex/skills"));
        dirs.push(home.join(".codex/skills/.system"));
        dirs.push(home.join(".claude/skills"));
    }
    if let Some(codex_home) = env::var_os("CODEX_HOME").map(PathBuf::from) {
        dirs.push(codex_home.join("skills"));
        dirs.push(codex_home.join("skills/.system"));
    }
    dirs
}

fn path_to_utf8(path: PathBuf) -> Result<Utf8PathBuf, DadrError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| DadrError::PathNotUtf8 {
        path: path.display().to_string(),
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_digest(digest.as_slice())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use camino::Utf8PathBuf;

    use super::{
        combined_prompt_pack_sha256, home_dir, list_prompt_packs, prompt_pack_search_dirs,
        resolve_prompt_packs,
    };

    #[test]
    fn resolves_project_skill_by_name() {
        let root = unique_temp_dir("dadrock-skill-name");
        let skill_dir = root.join(".agents/skills/dadr-style");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "Use terse DADR prose.\n").expect("write skill");
        let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");

        let packs = resolve_prompt_packs(&["dadr-style".to_owned()], &cwd).expect("resolve skill");

        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "dadr-style");
        assert!(packs[0]
            .source
            .ends_with(".agents/skills/dadr-style/SKILL.md"));
        assert_eq!(packs[0].sha256.len(), 64);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolves_installed_dadr_skill_by_name() {
        let root = unique_temp_dir("dadrock-installed-dadr");
        let skill_dir = root.join(".agents/skills/dadr");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "Project Dadrock skill.\n").expect("write skill");
        let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");

        let packs = resolve_prompt_packs(&["dadr".to_owned()], &cwd).expect("resolve skill");

        assert_eq!(packs.len(), 1);
        assert!(packs[0].source.ends_with(".agents/skills/dadr/SKILL.md"));
        assert_eq!(packs[0].instructions, "Project Dadrock skill.\n");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn combines_multiple_prompt_pack_hashes() {
        let root = unique_temp_dir("dadrock-skill-hash");
        fs::create_dir_all(&root).expect("create temp dir");
        let first = root.join("first.md");
        let second = root.join("second.md");
        fs::write(&first, "First.\n").expect("write first");
        fs::write(&second, "Second.\n").expect("write second");
        let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");

        let packs = resolve_prompt_packs(
            &[
                first.to_string_lossy().into_owned(),
                second.to_string_lossy().into_owned(),
            ],
            &cwd,
        )
        .expect("resolve packs");
        let combined = combined_prompt_pack_sha256(&packs).expect("combined hash");

        assert_eq!(combined.len(), 64);
        assert_ne!(combined, packs[0].sha256);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn search_dirs_include_codex_system_skills() {
        let Some(home) = home_dir() else {
            return;
        };
        let cwd = Utf8PathBuf::from("/tmp/dadrock-skill-search-test");
        let dirs = prompt_pack_search_dirs(&cwd);

        assert!(dirs.contains(&home.join(".codex/skills/.system")));
        assert!(dirs.contains(&home.join(".claude/skills")));
    }

    #[test]
    fn list_prompt_packs_does_not_invent_builtin_dadr() {
        let root = unique_temp_dir("dadrock-no-builtin-list");
        fs::create_dir_all(&root).expect("create temp dir");
        let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");

        let packs = list_prompt_packs(&cwd).expect("list packs");

        assert!(packs
            .iter()
            .all(|pack| !pack.source.as_str().starts_with("builtin:")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lists_prompt_packs_without_instructions() {
        let root = unique_temp_dir("dadrock-skill-list");
        let skill_dir = root.join(".agents/skills/dadr-style");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "Use terse DADR prose.\n").expect("write skill");
        let cwd = Utf8PathBuf::from_path_buf(root.clone()).expect("utf8 root");

        let packs = list_prompt_packs(&cwd).expect("list packs");

        let listed = packs
            .iter()
            .find(|pack| pack.name == "dadr-style")
            .expect("listed skill");
        assert!(listed
            .source
            .ends_with(".agents/skills/dadr-style/SKILL.md"));
        assert_eq!(listed.sha256.len(), 64);

        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ))
    }
}
