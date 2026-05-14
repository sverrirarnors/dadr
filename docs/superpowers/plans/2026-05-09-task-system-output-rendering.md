# Task System and Output Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Dadrock's extensible task system with built-in starter tasks, gitignored persisted runs, typed task outputs, and web UI renderers.

**Architecture:** Add a focused `dadrock_core::tasks` module for manifest parsing, task discovery, output validation, and run persistence. Expose that through the existing `commands` layer, then wire CLI, MCP, and web routes to those command functions. The web UI renders known output kinds with built-in components and falls back to a generic report/raw JSON view for unknown vendor kinds.

**Tech Stack:** Rust, `serde`, `serde_json`, `toml`, `camino`, existing ACP/MCP crates, current plain JavaScript/CSS preview UI.

---

## File Structure

Create these focused core files:

- `crates/dadrock-core/src/tasks/mod.rs` - public module surface and re-exports.
- `crates/dadrock-core/src/tasks/manifest.rs` - task manifest types, TOML parsing, manifest validation.
- `crates/dadrock-core/src/tasks/output.rs` - output kind parsing and result schema validation.
- `crates/dadrock-core/src/tasks/discovery.rs` - project/user/built-in task discovery and shadowing.
- `crates/dadrock-core/src/tasks/builtin.rs` - embedded built-in task package registry.
- `crates/dadrock-core/src/tasks/runs.rs` - persisted `.dadrock/runs/` layout and run metadata.
- `crates/dadrock-core/src/tasks/runner.rs` - ACP task prompt execution and output validation.
- `crates/dadrock-core/src/tasks/builtin/<task-id>/task.toml` and `prompt.md` - built-in task packages.
- `crates/dadrock-core/tests/task_system.rs` - core task tests.

Modify these existing files:

- `crates/dadrock-core/src/lib.rs` - expose `tasks`.
- `crates/dadrock-core/src/agent/types.rs` - add generic `RunTask` intent and task request payload.
- `crates/dadrock-core/src/agent/acp.rs` - raw JSON ACP task output helper.
- `crates/dadrock-core/src/agent/prompts.rs` - render generic task prompts.
- `crates/dadrock-core/src/commands/mod.rs` - command catalog and task command functions.
- `crates/dadrock-core/src/mcp.rs` - MCP task tools.
- `crates/dadrock/src/cli.rs` - `dadrock task ...` CLI parser.
- `crates/dadrock/src/main.rs` - CLI task handlers.
- `crates/dadrock/src/web.rs` - task API routes.
- `crates/dadrock/src/web/static/index.html` - add Tasks nav tab.
- `crates/dadrock/src/web/static/app.js` - task library, runner, run history, renderers.
- `crates/dadrock/src/web/static/app.css` - task view layout and result renderer styles.
- `docs-src/dadrock/cli.md`, `docs-src/dadrock/web.md`, `docs-src/llms.md` - user docs.

Use these existing commands for verification:

```bash
cargo test -p dadrock-core task_system
cargo test -p dadrock-core commands
cargo test -p dadrock mcp_stdio
node --check crates/dadrock/src/web/static/app.js
cargo test
```

---

### Task 1: Manifest and Output Types

**Files:**
- Create: `crates/dadrock-core/src/tasks/mod.rs`
- Create: `crates/dadrock-core/src/tasks/manifest.rs`
- Create: `crates/dadrock-core/src/tasks/output.rs`
- Create: `crates/dadrock-core/tests/task_system.rs`
- Modify: `crates/dadrock-core/src/lib.rs`

- [ ] **Step 1: Write failing manifest parse tests**

Add `crates/dadrock-core/tests/task_system.rs`:

```rust
use dadrock_core::tasks::{
    parse_task_manifest, TaskInputType, TaskOutputKind, TaskToolMode,
};

#[test]
fn task_manifest_parses_form_inputs_output_and_tools() {
    let manifest = parse_task_manifest(
        "builtin:find-conflicts",
        r#"
id = "find-conflicts"
title = "Find conflicting decisions"
description = "Scan live decisions and candidates for contradictions."
version = 1

[input]
mode = "form"

[[inputs]]
name = "status_scope"
label = "Records to scan"
type = "select"
default = "live"
options = ["live", "candidates", "accepted"]

[output]
kind = "conflict_set.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.list_records"
mode = "read"
"#,
    )
    .unwrap();

    assert_eq!(manifest.id, "find-conflicts");
    assert_eq!(manifest.version, 1);
    assert_eq!(manifest.inputs[0].field_type, TaskInputType::Select);
    assert_eq!(manifest.output.kind, TaskOutputKind::ConflictSetV1);
    assert_eq!(manifest.tools[0].mode, TaskToolMode::Read);
}

#[test]
fn task_manifest_rejects_invalid_ids() {
    let error = parse_task_manifest(
        "project:bad",
        r#"
id = "Bad Task"
title = "Bad"
description = "Bad id"
version = 1
[output]
kind = "markdown_report.v1"
[prompt]
system = "prompt.md"
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("task id"));
}

#[test]
fn vendor_output_kind_is_preserved_for_fallback_rendering() {
    let manifest = parse_task_manifest(
        "project:lab-audit",
        r#"
id = "lab-audit"
title = "Lab audit"
description = "Project-specific audit."
version = 1
[output]
kind = "lab.audit.v1"
[prompt]
system = "prompt.md"
"#,
    )
    .unwrap();

    assert_eq!(
        manifest.output.kind,
        TaskOutputKind::Vendor("lab.audit.v1".to_owned())
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: FAIL because `dadrock_core::tasks` does not exist.

- [ ] **Step 3: Add `tasks` module surface**

Add `crates/dadrock-core/src/tasks/mod.rs`:

```rust
pub mod manifest;
pub mod output;

pub use manifest::{
    parse_task_manifest, TaskInput, TaskInputMode, TaskInputType, TaskManifest, TaskOutput,
    TaskPrompt, TaskTool, TaskToolMode,
};
pub use output::{
    validate_task_output, CandidateBatchOutput, CandidateDraftOutput, ConflictSetOutput,
    DiagnosisReportOutput, MarkdownReportOutput, RecordRevisionOutput, SupersessionProposalOutput,
    TaskOutputKind, TaskOutputValue,
};
```

Modify `crates/dadrock-core/src/lib.rs`:

```rust
pub mod tasks;
```

Place the new line near the other public modules.

- [ ] **Step 4: Implement manifest types and parser**

Add `crates/dadrock-core/src/tasks/manifest.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::DadrError;

use super::output::TaskOutputKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskManifest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: u32,
    #[serde(default)]
    pub input: TaskInputModeConfig,
    #[serde(default)]
    pub inputs: Vec<TaskInput>,
    pub output: TaskOutput,
    pub prompt: TaskPrompt,
    #[serde(default)]
    pub tools: Vec<TaskTool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInputModeConfig {
    #[serde(default)]
    pub mode: TaskInputMode,
}

impl Default for TaskInputModeConfig {
    fn default() -> Self {
        Self {
            mode: TaskInputMode::Form,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskInputMode {
    Form,
}

impl Default for TaskInputMode {
    fn default() -> Self {
        Self::Form
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: TaskInputType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskInputType {
    Text,
    Textarea,
    Path,
    Select,
    Multiselect,
    Checkbox,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutput {
    pub kind: TaskOutputKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPrompt {
    pub system: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTool {
    pub name: String,
    pub mode: TaskToolMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskToolMode {
    Read,
    Write,
}

pub fn parse_task_manifest(source: &str, content: &str) -> Result<TaskManifest, DadrError> {
    let manifest: TaskManifest = toml::from_str(content).map_err(|err| {
        DadrError::InvalidInput(format!("failed to parse task manifest `{source}`: {err}"))
    })?;
    validate_manifest(source, &manifest)?;
    Ok(manifest)
}

fn validate_manifest(source: &str, manifest: &TaskManifest) -> Result<(), DadrError> {
    if !is_valid_task_id(&manifest.id) {
        return Err(DadrError::InvalidInput(format!(
            "task id `{}` in `{source}` must use lowercase letters, digits, and hyphens",
            manifest.id
        )));
    }
    if manifest.title.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must have a non-empty title",
            manifest.id
        )));
    }
    if manifest.description.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must have a non-empty description",
            manifest.id
        )));
    }
    if manifest.prompt.system.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must name a system prompt file",
            manifest.id
        )));
    }
    for input in &manifest.inputs {
        if !is_valid_input_name(&input.name) {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` input `{}` must use lowercase letters, digits, and underscores",
                manifest.id, input.name
            )));
        }
        if matches!(
            input.field_type,
            TaskInputType::Select | TaskInputType::Multiselect
        ) && input.options.is_empty()
        {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` input `{}` must declare options",
                manifest.id, input.name
            )));
        }
    }
    Ok(())
}

fn is_valid_task_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_valid_input_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
```

- [ ] **Step 5: Implement output kind parsing**

Add `crates/dadrock-core/src/tasks/output.rs`:

```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::DadrError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutputKind {
    CandidateDraftV1,
    CandidateBatchV1,
    ConflictSetV1,
    DiagnosisReportV1,
    SupersessionProposalV1,
    RecordRevisionV1,
    MarkdownReportV1,
    Vendor(String),
}

impl TaskOutputKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::CandidateDraftV1 => "candidate_draft.v1",
            Self::CandidateBatchV1 => "candidate_batch.v1",
            Self::ConflictSetV1 => "conflict_set.v1",
            Self::DiagnosisReportV1 => "diagnosis_report.v1",
            Self::SupersessionProposalV1 => "supersession_proposal.v1",
            Self::RecordRevisionV1 => "record_revision.v1",
            Self::MarkdownReportV1 => "markdown_report.v1",
            Self::Vendor(kind) => kind.as_str(),
        }
    }

    pub fn native_renderer(&self) -> bool {
        !matches!(self, Self::Vendor(_))
    }
}

impl Serialize for TaskOutputKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TaskOutputKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "candidate_draft.v1" => Self::CandidateDraftV1,
            "candidate_batch.v1" => Self::CandidateBatchV1,
            "conflict_set.v1" => Self::ConflictSetV1,
            "diagnosis_report.v1" => Self::DiagnosisReportV1,
            "supersession_proposal.v1" => Self::SupersessionProposalV1,
            "record_revision.v1" => Self::RecordRevisionV1,
            "markdown_report.v1" => Self::MarkdownReportV1,
            _ => Self::Vendor(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TaskOutputValue {
    #[serde(rename = "candidate_draft.v1")]
    CandidateDraft(CandidateDraftOutput),
    #[serde(rename = "candidate_batch.v1")]
    CandidateBatch(CandidateBatchOutput),
    #[serde(rename = "conflict_set.v1")]
    ConflictSet(ConflictSetOutput),
    #[serde(rename = "diagnosis_report.v1")]
    DiagnosisReport(DiagnosisReportOutput),
    #[serde(rename = "supersession_proposal.v1")]
    SupersessionProposal(SupersessionProposalOutput),
    #[serde(rename = "record_revision.v1")]
    RecordRevision(RecordRevisionOutput),
    #[serde(rename = "markdown_report.v1")]
    MarkdownReport(MarkdownReportOutput),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateDraftOutput {
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub body_markdown: String,
    #[serde(default)]
    pub source_references: Vec<Value>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateBatchOutput {
    pub summary: String,
    #[serde(default)]
    pub candidates: Vec<CandidateDraftProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateDraftProposal {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub body_markdown: String,
    #[serde(default)]
    pub source_references: Vec<Value>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictSetOutput {
    pub summary: String,
    #[serde(default)]
    pub findings: Vec<ConflictFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictFinding {
    pub id: String,
    pub title: String,
    pub left_record_id: String,
    pub right_record_id: String,
    pub relationship: String,
    pub conflict_explanation: String,
    pub why_might_not_be_conflict: String,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosisReportOutput {
    pub summary: String,
    #[serde(default)]
    pub issues: Vec<DiagnosisIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosisIssue {
    pub id: String,
    pub severity: String,
    pub category: String,
    #[serde(default)]
    pub record_id: Option<String>,
    pub title: String,
    pub explanation: String,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersessionProposalOutput {
    pub summary: String,
    pub old_record_id: String,
    pub new_record_id: String,
    pub rationale: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordRevisionOutput {
    pub record_id: String,
    pub proposed_title: String,
    #[serde(default)]
    pub proposed_tags: Vec<String>,
    pub proposed_body_markdown: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownReportOutput {
    pub summary: String,
    pub markdown: String,
}

pub fn validate_task_output(
    expected: &TaskOutputKind,
    value: Value,
) -> Result<Value, DadrError> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| DadrError::InvalidInput("task output must contain string field `kind`".to_owned()))?;
    if kind != expected.as_str() {
        if expected.native_renderer() {
            return Err(DadrError::InvalidInput(format!(
                "task output kind `{kind}` did not match expected `{}`",
                expected.as_str()
            )));
        }
        return Ok(value);
    }
    if expected.native_renderer() {
        serde_json::from_value::<TaskOutputValue>(value.clone()).map_err(|err| {
            DadrError::InvalidInput(format!("task output `{kind}` failed schema validation: {err}"))
        })?;
    }
    Ok(value)
}
```

- [ ] **Step 6: Run tests to verify Task 1 passes**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: PASS for the three tests added in this task.

- [ ] **Step 7: Commit Task 1**

```bash
git add crates/dadrock-core/src/lib.rs crates/dadrock-core/src/tasks/mod.rs crates/dadrock-core/src/tasks/manifest.rs crates/dadrock-core/src/tasks/output.rs crates/dadrock-core/tests/task_system.rs
git commit -m "feat: add task manifest and output types"
```

---

### Task 2: Built-In Task Packages and Discovery

**Files:**
- Create: `crates/dadrock-core/src/tasks/discovery.rs`
- Create: `crates/dadrock-core/src/tasks/builtin.rs`
- Create: `crates/dadrock-core/src/tasks/builtin/context-to-record/task.toml`
- Create: `crates/dadrock-core/src/tasks/builtin/context-to-record/prompt.md`
- Create: `crates/dadrock-core/src/tasks/builtin/code-to-record/task.toml`
- Create: `crates/dadrock-core/src/tasks/builtin/code-to-record/prompt.md`
- Create: `crates/dadrock-core/src/tasks/builtin/find-conflicts/task.toml`
- Create: `crates/dadrock-core/src/tasks/builtin/find-conflicts/prompt.md`
- Create: `crates/dadrock-core/src/tasks/builtin/supersede-record/task.toml`
- Create: `crates/dadrock-core/src/tasks/builtin/supersede-record/prompt.md`
- Create: `crates/dadrock-core/src/tasks/builtin/edit-record/task.toml`
- Create: `crates/dadrock-core/src/tasks/builtin/edit-record/prompt.md`
- Modify: `crates/dadrock-core/src/tasks/mod.rs`
- Modify: `crates/dadrock-core/tests/task_system.rs`

- [ ] **Step 1: Write failing discovery tests**

Append to `crates/dadrock-core/tests/task_system.rs`:

```rust
use std::fs;

use camino::Utf8PathBuf;
use dadrock_core::tasks::{discover_tasks, TaskSourceKind};

fn temp_task_root(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "dadrock-task-{label}-{}",
        dadrock_core::RecordId::new_v7()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn discovery_lists_built_in_starter_tasks() {
    let root = temp_task_root("builtin");
    let listing = discover_tasks(&Utf8PathBuf::from_path_buf(root.clone()).unwrap(), None).unwrap();
    let ids = listing
        .tasks
        .iter()
        .map(|task| task.manifest.id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"context-to-record"));
    assert!(ids.contains(&"code-to-record"));
    assert!(ids.contains(&"find-conflicts"));
    assert!(ids.contains(&"supersede-record"));
    assert!(ids.contains(&"edit-record"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_task_shadows_builtin_task_by_id() {
    let root = temp_task_root("shadow");
    let task_dir = root.join(".dadrock/tasks/find-conflicts");
    fs::create_dir_all(&task_dir).unwrap();
    fs::write(
        task_dir.join("task.toml"),
        r#"
id = "find-conflicts"
title = "Project conflict finder"
description = "Project-specific conflict task."
version = 7
[output]
kind = "conflict_set.v1"
[prompt]
system = "prompt.md"
"#,
    )
    .unwrap();
    fs::write(task_dir.join("prompt.md"), "Return conflict_set.v1 JSON.").unwrap();

    let listing = discover_tasks(&Utf8PathBuf::from_path_buf(root.clone()).unwrap(), None).unwrap();
    let task = listing
        .tasks
        .iter()
        .find(|task| task.manifest.id == "find-conflicts")
        .unwrap();

    assert_eq!(task.manifest.title, "Project conflict finder");
    assert_eq!(task.manifest.version, 7);
    assert_eq!(task.source.kind, TaskSourceKind::Project);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_project_task_is_reported_as_diagnostic() {
    let root = temp_task_root("invalid");
    let task_dir = root.join(".dadrock/tasks/bad-task");
    fs::create_dir_all(&task_dir).unwrap();
    fs::write(task_dir.join("task.toml"), "id = \"Bad Task\"\n").unwrap();

    let listing = discover_tasks(&Utf8PathBuf::from_path_buf(root.clone()).unwrap(), None).unwrap();

    assert!(listing
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("task id")
            || diagnostic.message.contains("missing field")));

    fs::remove_dir_all(root).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: FAIL because discovery and built-in registry are not implemented.

- [ ] **Step 3: Add built-in task registry**

Add `crates/dadrock-core/src/tasks/builtin.rs`:

```rust
pub struct BuiltinTaskPackage {
    pub id: &'static str,
    pub manifest: &'static str,
    pub prompt: &'static str,
}

pub const BUILTIN_TASKS: &[BuiltinTaskPackage] = &[
    BuiltinTaskPackage {
        id: "context-to-record",
        manifest: include_str!("builtin/context-to-record/task.toml"),
        prompt: include_str!("builtin/context-to-record/prompt.md"),
    },
    BuiltinTaskPackage {
        id: "code-to-record",
        manifest: include_str!("builtin/code-to-record/task.toml"),
        prompt: include_str!("builtin/code-to-record/prompt.md"),
    },
    BuiltinTaskPackage {
        id: "find-conflicts",
        manifest: include_str!("builtin/find-conflicts/task.toml"),
        prompt: include_str!("builtin/find-conflicts/prompt.md"),
    },
    BuiltinTaskPackage {
        id: "supersede-record",
        manifest: include_str!("builtin/supersede-record/task.toml"),
        prompt: include_str!("builtin/supersede-record/prompt.md"),
    },
    BuiltinTaskPackage {
        id: "edit-record",
        manifest: include_str!("builtin/edit-record/task.toml"),
        prompt: include_str!("builtin/edit-record/prompt.md"),
    },
];
```

- [ ] **Step 4: Add built-in task manifests**

Create the six built-in task packages. Use this exact `task.toml` for `context-to-record`:

```toml
id = "context-to-record"
title = "Draft candidate from context"
description = "Draft one candidate DADR from user-pasted context."
version = 1

[input]
mode = "form"

[[inputs]]
name = "context"
label = "Context"
type = "textarea"
required = true

[[inputs]]
name = "extra_instructions"
label = "Extra instructions"
type = "textarea"

[output]
kind = "candidate_draft.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.project_snapshot"
mode = "read"

[[tools]]
name = "dadrock.list_records"
mode = "read"
```

Use this `prompt.md`:

```markdown
You draft exactly one Data Analysis Decision Record candidate.

Use the provided context and the Dadrock project snapshot. Return exactly one
JSON object of kind `candidate_draft.v1`. The candidate must have a concise
title, `writer/agent`, one `kind/...` tag, and body markdown with the four
sections `Context`, `Decision`, `Alternatives`, and `Consequences`.

Do not mark the record accepted. Do not write files directly.
```

Create analogous task packages:

`code-to-record/task.toml`:

```toml
id = "code-to-record"
title = "Infer decisions from code"
description = "Inspect a code path and propose candidate DADRs for consequential choices."
version = 1

[input]
mode = "form"

[[inputs]]
name = "path"
label = "Code path"
type = "path"
default = "."
required = true

[[inputs]]
name = "max_candidates"
label = "Maximum candidates"
type = "number"
default = "10"

[output]
kind = "candidate_batch.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.project_snapshot"
mode = "read"

[[tools]]
name = "dadrock.list_records"
mode = "read"
```

`code-to-record/prompt.md`:

```markdown
Inspect the requested code path through the tools available to the host agent.
Propose candidate DADRs only for consequential analytic choices. Return exactly
one JSON object of kind `candidate_batch.v1`.

Each candidate must include a title, tags, body markdown with the four DADR
sections, confidence, warnings, and source references when available. Do not
write files directly.
```

`find-conflicts/task.toml`:

```toml
id = "find-conflicts"
title = "Find conflicting decisions"
description = "Scan live decisions and candidates for contradictions."
version = 1

[input]
mode = "form"

[[inputs]]
name = "status_scope"
label = "Records to scan"
type = "select"
default = "live"
options = ["live", "candidates", "accepted"]

[output]
kind = "conflict_set.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.list_records"
mode = "read"

[[tools]]
name = "dadrock.get_record"
mode = "read"

[[tools]]
name = "dadrock.list_tags"
mode = "read"
```

`find-conflicts/prompt.md`:

```markdown
Find pairs or clusters of DADR records whose Decision sections cannot both be
true on the same analytic surface. Return exactly one JSON object of kind
`conflict_set.v1`.

Use cheap filters first: live records, non-superseded accepted records,
candidates, tags, and title keywords. Read bodies only for plausible clusters.
For each finding include both record ids, the relationship, a concrete conflict
explanation, a reason it might not be a conflict, and suggested actions. Never
auto-resolve or mutate lifecycle state.
```

`supersede-record/task.toml`:

```toml
id = "supersede-record"
title = "Suggest supersession"
description = "Compare one candidate with similar records and suggest replacement direction."
version = 1

[[inputs]]
name = "candidate_query"
label = "Candidate"
type = "text"
required = true

[output]
kind = "supersession_proposal.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.get_record"
mode = "read"

[[tools]]
name = "dadrock.list_records"
mode = "read"
```

`supersede-record/prompt.md`:

```markdown
Compare the named candidate with similar accepted records and live candidates.
Return exactly one JSON object of kind `supersession_proposal.v1` describing the
likely old record, likely new record, rationale, and warnings.

Do not call lifecycle mutation tools. Supersession is advisory until the user
accepts records and runs the normal Dadrock supersede command.
```

`edit-record/task.toml`:

```toml
id = "edit-record"
title = "Revise candidate"
description = "Propose a full-body revision for an existing candidate."
version = 1

[[inputs]]
name = "candidate_query"
label = "Candidate"
type = "text"
required = true

[[inputs]]
name = "instruction"
label = "Revision instruction"
type = "textarea"
required = true

[output]
kind = "record_revision.v1"

[prompt]
system = "prompt.md"

[[tools]]
name = "dadrock.get_record"
mode = "read"
```

`edit-record/prompt.md`:

```markdown
Propose a complete revised candidate body. Return exactly one JSON object of
kind `record_revision.v1` with the record id, proposed title, proposed tags,
proposed body markdown, and summary.

Preserve the four standard sections. Do not revise accepted records.
```

- [ ] **Step 5: Implement discovery**

Modify `crates/dadrock-core/src/tasks/mod.rs`:

```rust
pub mod discovery;
pub mod manifest;
pub mod output;

mod builtin;

pub use discovery::{
    default_user_tasks_dir, discover_tasks, TaskDiscoveryDiagnostic, TaskDiscoveryDiagnosticCode, TaskListing,
    TaskPackage, TaskSource, TaskSourceKind,
};
pub use manifest::{
    parse_task_manifest, TaskInput, TaskInputMode, TaskInputType, TaskManifest, TaskOutput,
    TaskPrompt, TaskTool, TaskToolMode,
};
pub use output::{
    validate_task_output, CandidateBatchOutput, CandidateDraftOutput, ConflictSetOutput,
    DiagnosisReportOutput, MarkdownReportOutput, RecordRevisionOutput, SupersessionProposalOutput,
    TaskOutputKind, TaskOutputValue,
};
```

Add `crates/dadrock-core/src/tasks/discovery.rs`:

```rust
use std::{collections::BTreeMap, env, fs};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::DadrError;

use super::{
    builtin::BUILTIN_TASKS,
    manifest::{parse_task_manifest, TaskManifest},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskListing {
    pub tasks: Vec<TaskPackage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<TaskDiscoveryDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPackage {
    pub manifest: TaskManifest,
    pub source: TaskSource,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSource {
    pub kind: TaskSourceKind,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskSourceKind {
    Project,
    User,
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDiscoveryDiagnostic {
    pub code: TaskDiscoveryDiagnosticCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDiscoveryDiagnosticCode {
    InvalidManifest,
    MissingPrompt,
    ReadFailed,
}

pub fn discover_tasks(
    project_cwd: &Utf8Path,
    user_tasks_dir: Option<Utf8PathBuf>,
) -> Result<TaskListing, DadrError> {
    let mut tasks = BTreeMap::<String, TaskPackage>::new();
    let mut diagnostics = Vec::new();

    collect_dir_tasks(
        &mut tasks,
        &mut diagnostics,
        TaskSourceKind::Project,
        &project_cwd.join(".dadrock/tasks"),
    )?;

    if let Some(user_tasks_dir) = user_tasks_dir {
        collect_dir_tasks(
            &mut tasks,
            &mut diagnostics,
            TaskSourceKind::User,
            &user_tasks_dir,
        )?;
    }

    for builtin in BUILTIN_TASKS {
        if tasks.contains_key(builtin.id) {
            continue;
        }
        match parse_task_manifest(&format!("builtin:{}", builtin.id), builtin.manifest) {
            Ok(manifest) => {
                tasks.insert(
                    manifest.id.clone(),
                    TaskPackage {
                        source: TaskSource {
                            kind: TaskSourceKind::Builtin,
                            id: format!("builtin:{}", builtin.id),
                            path: None,
                        },
                        manifest,
                        prompt: builtin.prompt.to_owned(),
                    },
                );
            }
            Err(error) => diagnostics.push(TaskDiscoveryDiagnostic {
                code: TaskDiscoveryDiagnosticCode::InvalidManifest,
                message: error.to_string(),
                path: None,
            }),
        }
    }

    Ok(TaskListing {
        tasks: tasks.into_values().collect(),
        diagnostics,
    })
}

pub fn default_user_tasks_dir() -> Option<Utf8PathBuf> {
    env::var_os("DADROCK_TASKS_DIR")
        .map(Utf8PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Utf8PathBuf::from(home).join(".dadrock/tasks")))
}

fn collect_dir_tasks(
    tasks: &mut BTreeMap<String, TaskPackage>,
    diagnostics: &mut Vec<TaskDiscoveryDiagnostic>,
    kind: TaskSourceKind,
    dir: &Utf8Path,
) -> Result<(), DadrError> {
    let Ok(entries) = fs::read_dir(dir.as_std_path()) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(task_dir) = Utf8PathBuf::from_path_buf(path) else {
            continue;
        };
        let manifest_path = task_dir.join("task.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let manifest_content = match fs::read_to_string(manifest_path.as_std_path()) {
            Ok(content) => content,
            Err(error) => {
                diagnostics.push(TaskDiscoveryDiagnostic {
                    code: TaskDiscoveryDiagnosticCode::ReadFailed,
                    message: format!("failed to read `{manifest_path}`: {error}"),
                    path: Some(manifest_path),
                });
                continue;
            }
        };
        let manifest = match parse_task_manifest(manifest_path.as_str(), &manifest_content) {
            Ok(manifest) => manifest,
            Err(error) => {
                diagnostics.push(TaskDiscoveryDiagnostic {
                    code: TaskDiscoveryDiagnosticCode::InvalidManifest,
                    message: error.to_string(),
                    path: Some(manifest_path),
                });
                continue;
            }
        };
        if tasks.contains_key(&manifest.id) {
            continue;
        }
        let prompt_path = task_dir.join(&manifest.prompt.system);
        let prompt = match fs::read_to_string(prompt_path.as_std_path()) {
            Ok(prompt) => prompt,
            Err(error) => {
                diagnostics.push(TaskDiscoveryDiagnostic {
                    code: TaskDiscoveryDiagnosticCode::MissingPrompt,
                    message: format!("failed to read `{prompt_path}`: {error}"),
                    path: Some(prompt_path),
                });
                continue;
            }
        };
        tasks.insert(
            manifest.id.clone(),
            TaskPackage {
                source: TaskSource {
                    kind,
                    id: format!("{}:{}", source_kind_label(kind), manifest.id),
                    path: Some(task_dir),
                },
                manifest,
                prompt,
            },
        );
    }
    Ok(())
}

fn source_kind_label(kind: TaskSourceKind) -> &'static str {
    match kind {
        TaskSourceKind::Project => "project",
        TaskSourceKind::User => "user",
        TaskSourceKind::Builtin => "builtin",
    }
}
```

- [ ] **Step 6: Run discovery tests**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: PASS for manifest and discovery tests.

- [ ] **Step 7: Commit Task 2**

```bash
git add crates/dadrock-core/src/tasks/mod.rs crates/dadrock-core/src/tasks/discovery.rs crates/dadrock-core/src/tasks/builtin.rs crates/dadrock-core/src/tasks/builtin crates/dadrock-core/tests/task_system.rs
git commit -m "feat: discover built-in and project tasks"
```

---

### Task 3: Persisted Task Runs

**Files:**
- Create: `crates/dadrock-core/src/tasks/runs.rs`
- Modify: `crates/dadrock-core/src/tasks/mod.rs`
- Modify: `crates/dadrock-core/tests/task_system.rs`

- [ ] **Step 1: Write failing run persistence tests**

Append to `crates/dadrock-core/tests/task_system.rs`:

```rust
use dadrock_core::tasks::{
    create_task_run, read_task_run, task_runs_dir, write_task_result, TaskRunStatus,
};
use serde_json::json;

#[test]
fn create_task_run_persists_gitignored_local_state() {
    let root = temp_task_root("runs");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let run = create_task_run(&cwd, "find-conflicts", json!({"status_scope": "live"})).unwrap();

    assert_eq!(run.task_id, "find-conflicts");
    assert_eq!(run.status, TaskRunStatus::Created);
    assert!(run.run_dir.is_dir());
    assert!(run.run_dir.join("run.json").is_file());
    assert!(run.run_dir.join("input.json").is_file());
    assert!(root.join(".dadrock/.gitignore").is_file());

    let gitignore = fs::read_to_string(root.join(".dadrock/.gitignore")).unwrap();
    assert!(gitignore.contains("/runs/"));
    assert!(gitignore.contains("!/tasks/"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn write_task_result_marks_run_succeeded_and_stores_result() {
    let root = temp_task_root("result");
    let cwd = Utf8PathBuf::from_path_buf(root.clone()).unwrap();
    let run = create_task_run(&cwd, "context-to-record", json!({"context": "Use HC3"})).unwrap();

    let updated = write_task_result(
        &run.run_dir,
        json!({
            "kind": "candidate_draft.v1",
            "title": "Use HC3 robust standard errors",
            "tags": ["kind/inference", "writer/agent"],
            "body_markdown": "## Context\n\nx\n\n## Decision\n\nx\n\n## Alternatives\n\nx\n\n## Consequences\n\nx",
            "source_references": [],
            "warnings": []
        }),
    )
    .unwrap();

    assert_eq!(updated.status, TaskRunStatus::Succeeded);
    assert!(run.run_dir.join("result.json").is_file());

    let reloaded = read_task_run(&run.run_dir).unwrap();
    assert_eq!(reloaded.status, TaskRunStatus::Succeeded);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn task_runs_dir_uses_dot_dadrock_runs() {
    let root = Utf8PathBuf::from_path_buf(temp_task_root("runs-dir")).unwrap();
    assert_eq!(task_runs_dir(&root), root.join(".dadrock/runs"));
    fs::remove_dir_all(root).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: FAIL because `runs.rs` has no implementation.

- [ ] **Step 3: Implement run persistence**

Modify `crates/dadrock-core/src/tasks/mod.rs` by adding:

```rust
pub mod runs;

pub use runs::{
    create_task_run, read_task_run, task_runs_dir, write_task_result, PersistedTaskRun,
    TaskRunStatus,
};
```

Add `crates/dadrock-core/src/tasks/runs.rs`:

```rust
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{DadrError, RecordId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedTaskRun {
    pub id: String,
    pub task_id: String,
    pub status: TaskRunStatus,
    pub created_at: String,
    pub updated_at: String,
    pub run_dir: Utf8PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRunStatus {
    Created,
    Running,
    Succeeded,
    Failed,
}

pub fn task_runs_dir(project_cwd: &Utf8Path) -> Utf8PathBuf {
    project_cwd.join(".dadrock/runs")
}

pub fn create_task_run(
    project_cwd: &Utf8Path,
    task_id: &str,
    input: Value,
) -> Result<PersistedTaskRun, DadrError> {
    ensure_dadrock_gitignore(project_cwd)?;
    let now = now_string();
    let run_id = format!(
        "{}-{}-{}",
        now.replace([':', '-', '.'], ""),
        task_id,
        RecordId::new_v7()
            .as_uuid()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );
    let run_dir = task_runs_dir(project_cwd).join(&run_id);
    fs::create_dir_all(run_dir.join("artifacts").as_std_path())?;
    let run = PersistedTaskRun {
        id: run_id,
        task_id: task_id.to_owned(),
        status: TaskRunStatus::Created,
        created_at: now.clone(),
        updated_at: now,
        run_dir,
    };
    write_json(&run.run_dir.join("run.json"), &run)?;
    write_json(&run.run_dir.join("input.json"), &input)?;
    Ok(run)
}

pub fn read_task_run(run_dir: &Utf8Path) -> Result<PersistedTaskRun, DadrError> {
    let content = fs::read_to_string(run_dir.join("run.json").as_std_path())?;
    serde_json::from_str(&content).map_err(Into::into)
}

pub fn write_task_result(run_dir: &Utf8Path, result: Value) -> Result<PersistedTaskRun, DadrError> {
    write_json(&run_dir.join("result.json"), &result)?;
    let mut run = read_task_run(run_dir)?;
    run.status = TaskRunStatus::Succeeded;
    run.updated_at = now_string();
    write_json(&run_dir.join("run.json"), &run)?;
    Ok(run)
}

fn ensure_dadrock_gitignore(project_cwd: &Utf8Path) -> Result<(), DadrError> {
    let dadrock_dir = project_cwd.join(".dadrock");
    fs::create_dir_all(dadrock_dir.as_std_path())?;
    let gitignore = dadrock_dir.join(".gitignore");
    if gitignore.exists() {
        return Ok(());
    }
    fs::write(
        gitignore.as_std_path(),
        "/runs/\n/cache/\n/tmp/\n!/tasks/\n!/.gitignore\n",
    )?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Utf8Path, value: &T) -> Result<(), DadrError> {
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path.as_std_path(), content)?;
    Ok(())
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
```

- [ ] **Step 4: Run run persistence tests**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: PASS.

- [ ] **Step 5: Commit Task 3**

```bash
git add crates/dadrock-core/src/tasks/mod.rs crates/dadrock-core/src/tasks/runs.rs crates/dadrock-core/tests/task_system.rs
git commit -m "feat: persist local task runs"
```

---

### Task 4: Core Commands and Command Catalog

**Files:**
- Modify: `crates/dadrock-core/src/commands/mod.rs`
- Modify: `crates/dadrock-core/tests/task_system.rs`

- [ ] **Step 1: Write failing command tests**

Append to `crates/dadrock-core/tests/task_system.rs`:

```rust
use dadrock_core::commands::{
    command_catalog, copy_task_command, list_tasks_command, show_task_command, CommandContext,
};

#[test]
fn command_catalog_includes_task_surface() {
    let catalog = command_catalog();
    let names = catalog
        .data
        .commands
        .iter()
        .map(|command| command.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"task"));
    assert!(names.contains(&"task list"));
    assert!(names.contains(&"task show"));
    assert!(names.contains(&"task runs"));
    assert!(names.contains(&"task result"));
}

#[test]
fn list_tasks_command_returns_builtins() {
    let root = temp_task_root("command-list");
    let context = CommandContext::from_path(&root).unwrap();

    let envelope = list_tasks_command(&context).unwrap();

    assert_eq!(envelope.command, "task list");
    assert!(envelope.ok);
    assert!(envelope
        .data
        .tasks
        .iter()
        .any(|task| task.manifest.id == "find-conflicts"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn show_task_command_resolves_one_task() {
    let root = temp_task_root("command-show");
    let context = CommandContext::from_path(&root).unwrap();

    let envelope = show_task_command(&context, "context-to-record").unwrap();

    assert_eq!(envelope.command, "task show");
    assert_eq!(envelope.data.manifest.id, "context-to-record");
    assert!(envelope.data.prompt.contains("candidate"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn copy_task_command_writes_builtin_to_project_tasks() {
    let root = temp_task_root("command-copy");
    let context = CommandContext::from_path(&root).unwrap();

    let envelope = copy_task_command(&context, "find-conflicts", None).unwrap();

    assert_eq!(envelope.command, "task copy");
    assert_eq!(envelope.data.task_id, "find-conflicts");
    assert!(root
        .join(".dadrock/tasks/find-conflicts/task.toml")
        .is_file());
    assert!(root
        .join(".dadrock/tasks/find-conflicts/prompt.md")
        .is_file());

    fs::remove_dir_all(root).unwrap();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: FAIL because task command functions and catalog entries are missing.

- [ ] **Step 3: Add command catalog entries**

Modify `crates/dadrock-core/src/commands/mod.rs`. Add imports:

```rust
use crate::tasks::{default_user_tasks_dir, discover_tasks, TaskListing, TaskPackage};
```

Add these `CommandInfo` entries after the `agent skills` entry and before `preview`:

```rust
CommandInfo {
    path: &["task"],
    usage: "task <list|show|run|runs|result|copy> [options]",
    description: "Run and inspect Dadrock task workflows.",
    mutation: CommandMutation::Writes,
    supports_json: false,
    exit_behavior: "Exits non-zero when the subcommand is missing or unknown.",
    subcommands: &["task list", "task show", "task run", "task runs", "task result", "task copy"],
},
CommandInfo {
    path: &["task", "list"],
    usage: "task list [--json]",
    description: "List built-in, project, and user Dadrock tasks.",
    mutation: CommandMutation::ReadOnly,
    supports_json: true,
    exit_behavior: "Exits non-zero when task discovery cannot inspect the project.",
    subcommands: &[],
},
CommandInfo {
    path: &["task", "show"],
    usage: "task show <task-id> [--json]",
    description: "Show one task manifest and prompt.",
    mutation: CommandMutation::ReadOnly,
    supports_json: true,
    exit_behavior: "Exits non-zero when the task id is missing or unresolved.",
    subcommands: &[],
},
CommandInfo {
    path: &["task", "run"],
    usage: "task run <task-id> [--json] [--input-json <json>]",
    description: "Run a Dadrock task through a configured ACP agent.",
    mutation: CommandMutation::Writes,
    supports_json: true,
    exit_behavior: "Exits non-zero when the task, input, agent, or output schema is invalid.",
    subcommands: &[],
},
CommandInfo {
    path: &["task", "runs"],
    usage: "task runs [--json]",
    description: "List persisted local task runs.",
    mutation: CommandMutation::ReadOnly,
    supports_json: true,
    exit_behavior: "Exits non-zero when local task run metadata cannot be read.",
    subcommands: &[],
},
CommandInfo {
    path: &["task", "result"],
    usage: "task result <run-id> [--json]",
    description: "Show one persisted task run result.",
    mutation: CommandMutation::ReadOnly,
    supports_json: true,
    exit_behavior: "Exits non-zero when the run id is missing or unresolved.",
    subcommands: &[],
},
CommandInfo {
    path: &["task", "copy"],
    usage: "task copy <task-id> [--to <directory>]",
    description: "Copy a built-in or user task into the project task directory for customization.",
    mutation: CommandMutation::Writes,
    supports_json: true,
    exit_behavior: "Exits non-zero when the task cannot be found or the destination is invalid.",
    subcommands: &[],
},
```

- [ ] **Step 4: Add list and show command functions**

Add near the agent command functions in `commands/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyTaskResult {
    pub task_id: String,
    pub task_dir: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub prompt_path: Utf8PathBuf,
}

pub fn list_tasks_command(
    context: &CommandContext,
) -> Result<CommandEnvelope<TaskListing>, DadrError> {
    let listing = discover_tasks(&context.cwd, default_user_tasks_dir())?;
    Ok(CommandEnvelope::new("task list", listing))
}

pub fn show_task_command(
    context: &CommandContext,
    task_id: &str,
) -> Result<CommandEnvelope<TaskPackage>, DadrError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(DadrError::InvalidInput("task id cannot be empty".to_owned()));
    }
    let listing = discover_tasks(&context.cwd, default_user_tasks_dir())?;
    let task = listing
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == task_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task `{task_id}` was not found")))?;
    Ok(CommandEnvelope::new("task show", task))
}

pub fn copy_task_command(
    context: &CommandContext,
    task_id: &str,
    target: Option<Utf8PathBuf>,
) -> Result<CommandEnvelope<CopyTaskResult>, DadrError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(DadrError::InvalidInput("task id cannot be empty".to_owned()));
    }
    let listing = discover_tasks(&context.cwd, default_user_tasks_dir())?;
    let task = listing
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == task_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task `{task_id}` was not found")))?;
    let task_dir = target.unwrap_or_else(|| context.cwd.join(".dadrock/tasks").join(task_id));
    fs::create_dir_all(task_dir.as_std_path())?;
    let manifest_path = task_dir.join("task.toml");
    let prompt_path = task_dir.join(&task.manifest.prompt.system);
    let manifest = toml::to_string_pretty(&task.manifest).map_err(|err| {
        DadrError::InvalidInput(format!("failed to serialize task `{task_id}`: {err}"))
    })?;
    fs::write(manifest_path.as_std_path(), manifest)?;
    fs::write(prompt_path.as_std_path(), task.prompt)?;
    let result = CopyTaskResult {
        task_id: task_id.to_owned(),
        task_dir,
        manifest_path,
        prompt_path,
    };
    let changed_paths = vec![result.manifest_path.clone(), result.prompt_path.clone()];
    Ok(CommandEnvelope::new("task copy", result).with_changed_paths(changed_paths))
}
```

- [ ] **Step 5: Update command catalog tests**

In `commands/mod.rs`, update the existing command catalog tests so the expected path list includes:

```rust
&["task"],
&["task", "list"],
&["task", "show"],
&["task", "run"],
&["task", "runs"],
&["task", "result"],
&["task", "copy"],
```

Also update mutation and JSON support expectations so:

- `task`, `task run`, and `task copy` are mutating.
- `task list`, `task show`, `task runs`, and `task result` are read-only.
- all task subcommands except parent `task` support JSON.

- [ ] **Step 6: Run command tests**

Run:

```bash
cargo test -p dadrock-core task_system
cargo test -p dadrock-core commands
```

Expected: PASS.

- [ ] **Step 7: Commit Task 4**

```bash
git add crates/dadrock-core/src/commands/mod.rs crates/dadrock-core/tests/task_system.rs
git commit -m "feat: expose task catalog commands"
```

---

### Task 5: Generic ACP Task Runner

**Files:**
- Create: `crates/dadrock-core/src/tasks/runner.rs`
- Modify: `crates/dadrock-core/src/tasks/mod.rs`
- Modify: `crates/dadrock-core/src/agent/acp.rs`
- Modify: `crates/dadrock-core/src/agent/types.rs`
- Modify: `crates/dadrock-core/src/agent/prompts.rs`
- Modify: `crates/dadrock-core/src/commands/mod.rs`
- Modify: `crates/dadrock-core/tests/task_system.rs`

- [ ] **Step 1: Write failing output validation tests for runner**

Append to `crates/dadrock-core/tests/task_system.rs`:

```rust
use dadrock_core::tasks::{validate_task_output, TaskOutputKind};

#[test]
fn validate_task_output_accepts_matching_candidate_draft() {
    let value = validate_task_output(
        &TaskOutputKind::CandidateDraftV1,
        json!({
            "kind": "candidate_draft.v1",
            "title": "Use HC3 robust standard errors",
            "tags": ["kind/inference", "writer/agent"],
            "body_markdown": "## Context\n\nx\n\n## Decision\n\nx\n\n## Alternatives\n\nx\n\n## Consequences\n\nx",
            "source_references": [],
            "warnings": []
        }),
    )
    .unwrap();

    assert_eq!(value["kind"], "candidate_draft.v1");
}

#[test]
fn validate_task_output_rejects_wrong_native_kind() {
    let error = validate_task_output(
        &TaskOutputKind::ConflictSetV1,
        json!({
            "kind": "candidate_draft.v1",
            "title": "Wrong",
            "tags": [],
            "body_markdown": "## Context\n\nx",
            "source_references": [],
            "warnings": []
        }),
    )
    .unwrap_err();

    assert!(error.to_string().contains("did not match expected"));
}

#[test]
fn validate_task_output_keeps_vendor_kind_for_fallback() {
    let value = validate_task_output(
        &TaskOutputKind::Vendor("lab.audit.v1".to_owned()),
        json!({
            "kind": "lab.audit.v1",
            "summary": "Lab-specific result",
            "items": []
        }),
    )
    .unwrap();

    assert_eq!(value["kind"], "lab.audit.v1");
}
```

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test -p dadrock-core task_system
```

Expected: PASS. This confirms the output validator from Task 1 accepts native schemas, rejects mismatched native kinds, and preserves vendor kinds for fallback rendering.

- [ ] **Step 3: Extend agent intent and request envelope**

Modify `crates/dadrock-core/src/agent/types.rs`.

Add this enum variant:

```rust
RunTask,
```

Update `AgentIntent::as_str`:

```rust
Self::RunTask => "run_task",
```

Add these structs:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTaskRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub output_kind: String,
    pub input: serde_json::Value,
    pub prompt: String,
}
```

Add this optional field to `AgentRequestEnvelope`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub task: Option<AgentTaskRequest>,
```

Update existing constructors to set `task: None`.

Add this constructor:

```rust
pub fn run_task(
    prompt: String,
    project: AgentProjectSnapshot,
    task: AgentTaskRequest,
) -> Result<Self, crate::error::DadrError> {
    Ok(Self {
        protocol: DADROCK_DRAFT_PROTOCOL.to_owned(),
        version: DADROCK_DRAFT_VERSION,
        intent: AgentIntent::RunTask,
        prompt: bounded_user_prompt(prompt)?,
        project,
        prompt_packs: Vec::new(),
        candidate: None,
        chat_history: Vec::new(),
        task: Some(task),
    })
}
```

- [ ] **Step 4: Render generic task prompt**

Modify `crates/dadrock-core/src/agent/prompts.rs`.

Update `agent_prompt`:

```rust
AgentIntent::RunTask => run_task_prompt(request),
```

Add:

```rust
pub fn run_task_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let request_json = serde_json::to_string_pretty(request)?;
    let output_kind = request
        .task
        .as_ref()
        .map(|task| task.output_kind.as_str())
        .unwrap_or("markdown_report.v1");
    Ok(format!(
        r###"You are running a Dadrock task.

Return exactly one JSON object and no prose outside it. The JSON object must
have `kind` equal to `{output_kind}`. Follow the task prompt and use Dadrock MCP
tools when available. Do not bypass Dadrock lifecycle commands.

Dadrock task request:
```json
{request_json}
```
"###
    ))
}
```

- [ ] **Step 5: Add raw JSON ACP helper**

Modify `crates/dadrock-core/src/agent/acp.rs` by adding this public helper next to `run_acp_agent`:

```rust
pub fn run_acp_agent_json(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
) -> Result<serde_json::Value, DadrError> {
    let prompt = agent_prompt(request)?;
    let text = run_acp_prompt(
        command,
        request.project.cwd.clone().into_std_path_buf(),
        &prompt,
        options,
    )?;
    serde_json::from_str(text.trim()).map_err(|err| {
        DadrError::Acp(format!(
            "ACP agent task response was not a JSON object matching the task output contract: {err}"
        ))
    })
}
```

This helper intentionally returns raw JSON instead of `AgentResponseEnvelope`.
Task outputs are typed by `kind`, not by the candidate draft envelope.

- [ ] **Step 6: Implement runner command request**

Modify `crates/dadrock-core/src/tasks/mod.rs` by adding:

```rust
pub mod runner;

pub use runner::{run_task_with_acp, TaskRunCommandRequest};
```

Add to `crates/dadrock-core/src/tasks/runner.rs`:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    agent::{
        acp::{run_acp_agent_json, AcpPermissionPolicy, AcpPromptOptions},
        providers::LocalAcpAgentCommand,
        types::{AgentProjectSnapshot, AgentRequestEnvelope, AgentTaskRequest},
    },
    index::ProjectIndex,
    project::ProjectContext,
    tasks::{create_task_run, default_user_tasks_dir, discover_tasks, validate_task_output, write_task_result},
    DadrError,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRunCommandRequest {
    pub task_id: String,
    pub input: Value,
    pub local_agent: LocalAcpAgentCommand,
    pub permission_policy: AcpPermissionPolicy,
    #[serde(default = "default_enable_dadrock_mcp")]
    pub enable_dadrock_mcp: bool,
}

fn default_enable_dadrock_mcp() -> bool {
    true
}

pub fn run_task_with_acp(
    project: &ProjectContext,
    request: TaskRunCommandRequest,
) -> Result<super::runs::PersistedTaskRun, DadrError> {
    let listing = discover_tasks(&project.cwd, default_user_tasks_dir())?;
    let task = listing
        .tasks
        .into_iter()
        .find(|task| task.manifest.id == request.task_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task `{}` was not found", request.task_id)))?;
    let run = create_task_run(&project.cwd, &task.manifest.id, request.input.clone())?;
    let index = ProjectIndex::scan(project)?;
    let records = index.list_records(None);
    let agent_request = AgentRequestEnvelope::run_task(
        task.prompt.clone(),
        AgentProjectSnapshot::new(project, records),
        AgentTaskRequest {
            id: task.manifest.id.clone(),
            title: task.manifest.title.clone(),
            description: task.manifest.description.clone(),
            output_kind: task.manifest.output.kind.as_str().to_owned(),
            input: request.input,
            prompt: task.prompt,
        },
    )?;
    let output = run_acp_agent_json(
        &request.local_agent,
        &agent_request,
        AcpPromptOptions {
            permission_policy: request.permission_policy,
            enable_dadrock_mcp: request.enable_dadrock_mcp,
            ..AcpPromptOptions::default()
        },
    )?;
    let validated = validate_task_output(&task.manifest.output.kind, output)?;
    write_task_result(&run.run_dir, validated)
}
```

- [ ] **Step 7: Add command wrapper**

Modify `commands/mod.rs` imports:

```rust
use crate::tasks::{run_task_with_acp, PersistedTaskRun, TaskRunCommandRequest};
```

Add:

```rust
pub fn run_task_command(
    context: &CommandContext,
    request: TaskRunCommandRequest,
) -> Result<CommandEnvelope<PersistedTaskRun>, DadrError> {
    let project = context.discover_project()?;
    project.require_initialized()?;
    let run = run_task_with_acp(&project, request)?;
    Ok(CommandEnvelope::new("task run", run))
}
```

- [ ] **Step 8: Run runner tests and build**

Run:

```bash
cargo test -p dadrock-core task_system
cargo build -p dadrock-core
```

Expected: PASS and build succeeds.

- [ ] **Step 9: Commit Task 5**

```bash
git add crates/dadrock-core/src/tasks/mod.rs crates/dadrock-core/src/tasks/runner.rs crates/dadrock-core/src/agent/acp.rs crates/dadrock-core/src/agent/types.rs crates/dadrock-core/src/agent/prompts.rs crates/dadrock-core/src/commands/mod.rs crates/dadrock-core/tests/task_system.rs
git commit -m "feat: run tasks through ACP"
```

---

### Task 6: CLI Task Surface

**Files:**
- Modify: `crates/dadrock/src/cli.rs`
- Modify: `crates/dadrock/src/main.rs`
- Modify: `crates/dadrock-core/src/tasks/mod.rs`
- Modify: `crates/dadrock-core/src/tasks/runs.rs`
- Modify: `crates/dadrock-core/src/commands/mod.rs`

- [ ] **Step 1: Write failing CLI parser tests**

In `crates/dadrock/src/cli.rs`, add tests:

```rust
#[test]
fn parses_task_list_json() {
    let cli = Cli::try_parse_from(["dadrock", "task", "list", "--json"]).unwrap();
    let Command::Task(TaskArgs {
        command: TaskCommand::List(args),
    }) = cli.into_command_or_default()
    else {
        panic!("expected task list");
    };
    assert!(args.json);
}

#[test]
fn parses_task_show() {
    let cli = Cli::try_parse_from(["dadrock", "task", "show", "find-conflicts"]).unwrap();
    let Command::Task(TaskArgs {
        command: TaskCommand::Show(args),
    }) = cli.into_command_or_default()
    else {
        panic!("expected task show");
    };
    assert_eq!(args.task_id, "find-conflicts");
}

#[test]
fn parses_task_run_with_input_json() {
    let cli = Cli::try_parse_from([
        "dadrock",
        "task",
        "run",
        "find-conflicts",
        "--input-json",
        "{\"status_scope\":\"live\"}",
        "--json",
    ])
    .unwrap();
    let Command::Task(TaskArgs {
        command: TaskCommand::Run(args),
    }) = cli.into_command_or_default()
    else {
        panic!("expected task run");
    };
    assert_eq!(args.task_id, "find-conflicts");
    assert!(args.json);
    assert_eq!(args.input_json, "{\"status_scope\":\"live\"}");
}

#[test]
fn parses_task_copy() {
    let cli = Cli::try_parse_from([
        "dadrock",
        "task",
        "copy",
        "find-conflicts",
        "--to",
        ".dadrock/tasks/find-conflicts",
        "--json",
    ])
    .unwrap();
    let Command::Task(TaskArgs {
        command: TaskCommand::Copy(args),
    }) = cli.into_command_or_default()
    else {
        panic!("expected task copy");
    };
    assert_eq!(args.task_id, "find-conflicts");
    assert_eq!(args.to, Some(Utf8PathBuf::from(".dadrock/tasks/find-conflicts")));
    assert!(args.json);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p dadrock parses_task
```

Expected: FAIL because CLI task types are absent.

- [ ] **Step 3: Add CLI task structs**

Modify `crates/dadrock/src/cli.rs`. Add `Task(TaskArgs)` to `Command`:

```rust
#[command(about = "Run and inspect Dadrock task workflows.")]
Task(TaskArgs),
```

Add:

```rust
#[derive(Debug, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    #[command(about = "List built-in, project, and user Dadrock tasks.")]
    List(JsonArg),
    #[command(about = "Show one Dadrock task.")]
    Show(TaskShowArgs),
    #[command(about = "Run one Dadrock task through the configured ACP agent.")]
    Run(TaskRunArgs),
    #[command(about = "List persisted local task runs.")]
    Runs(JsonArg),
    #[command(about = "Show one task run result.")]
    Result(TaskResultArgs),
    #[command(about = "Copy one task into the project task directory.")]
    Copy(TaskCopyArgs),
}

#[derive(Debug, Args)]
pub struct TaskShowArgs {
    pub task_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TaskRunArgs {
    pub task_id: String,
    #[arg(long, default_value = "{}")]
    pub input_json: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long, value_enum, default_value_t = PermissionPolicyArg::Deny)]
    pub permission: PermissionPolicyArg,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "agent-arg")]
    pub agent_args: Vec<String>,
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

#[derive(Debug, Args)]
pub struct TaskResultArgs {
    pub run_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TaskCopyArgs {
    pub task_id: String,
    #[arg(long)]
    pub to: Option<Utf8PathBuf>,
    #[arg(long)]
    pub json: bool,
}
```

- [ ] **Step 4: Add main handlers**

Modify `crates/dadrock/src/main.rs`. In the top-level match, add:

```rust
cli::Command::Task(args) => task(args),
```

Add handler functions:

```rust
fn task(args: cli::TaskArgs) -> Result<ExitCode> {
    match args.command {
        cli::TaskCommand::List(args) => task_list(args),
        cli::TaskCommand::Show(args) => task_show(args),
        cli::TaskCommand::Run(args) => task_run(args),
        cli::TaskCommand::Runs(args) => task_runs(args),
        cli::TaskCommand::Result(args) => task_result(args),
        cli::TaskCommand::Copy(args) => task_copy(args),
    }
}

fn task_list(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_tasks_command(&command_context()?)?;
    if args.json {
        print_json(&envelope)?;
    } else {
        for task in envelope.data.tasks {
            println!(
                "{}\t{}\t{:?}",
                task.manifest.id, task.manifest.title, task.source.kind
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn task_show(args: cli::TaskShowArgs) -> Result<ExitCode> {
    let envelope = core_commands::show_task_command(&command_context()?, &args.task_id)?;
    if args.json {
        print_json(&envelope)?;
    } else {
        println!("{}\n\n{}", envelope.data.manifest.title, envelope.data.prompt);
    }
    Ok(ExitCode::SUCCESS)
}

fn task_run(args: cli::TaskRunArgs) -> Result<ExitCode> {
    let input = serde_json::from_str(&args.input_json)
        .with_context(|| "failed to parse --input-json as JSON")?;
    let local_agent = local_acp_agent_command(args.agent, args.agent_args)?;
    let envelope = core_commands::run_task_command(
        &command_context()?,
        dadrock_core::tasks::TaskRunCommandRequest {
            task_id: args.task_id,
            input,
            local_agent,
            permission_policy: args.permission.into(),
            enable_dadrock_mcp: !args.no_mcp,
        },
    )?;
    if args.json {
        print_json(&envelope)?;
    } else {
        println!("task run {} {}", envelope.data.id, envelope.data.status.as_str());
    }
    Ok(ExitCode::SUCCESS)
}

fn task_runs(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_task_runs_command(&command_context()?)?;
    if args.json {
        print_json(&envelope)?;
    } else {
        for run in envelope.data {
            println!("{}\t{}\t{:?}", run.id, run.task_id, run.status);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn task_result(args: cli::TaskResultArgs) -> Result<ExitCode> {
    let envelope = core_commands::show_task_result_command(&command_context()?, &args.run_id)?;
    if args.json {
        print_json(&envelope)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&envelope.data)?);
    }
    Ok(ExitCode::SUCCESS)
}

fn task_copy(args: cli::TaskCopyArgs) -> Result<ExitCode> {
    let envelope = core_commands::copy_task_command(&command_context()?, &args.task_id, args.to)?;
    if args.json {
        print_json(&envelope)?;
    } else {
        println!("copied task {} to {}", envelope.data.task_id, envelope.data.task_dir);
    }
    Ok(ExitCode::SUCCESS)
}
```

Add `TaskRunStatus::as_str` in `runs.rs`:

```rust
impl TaskRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}
```

- [ ] **Step 5: Add list run and result command functions**

In `runs.rs`, add functions to list run directories and read result JSON:

```rust
pub fn list_task_runs(project_cwd: &Utf8Path) -> Result<Vec<PersistedTaskRun>, DadrError> {
    let dir = task_runs_dir(project_cwd);
    let Ok(entries) = fs::read_dir(dir.as_std_path()) else {
        return Ok(Vec::new());
    };
    let mut runs = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        if let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) {
            if let Ok(run) = read_task_run(&path) {
                runs.push(run);
            }
        }
    }
    runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(runs)
}

pub fn read_task_result(run_dir: &Utf8Path) -> Result<Value, DadrError> {
    let content = fs::read_to_string(run_dir.join("result.json").as_std_path())?;
    serde_json::from_str(&content).map_err(Into::into)
}
```

Modify `crates/dadrock-core/src/tasks/mod.rs` so the `runs` re-export includes the new functions:

```rust
pub use runs::{
    create_task_run, list_task_runs, read_task_result, read_task_run, task_runs_dir,
    write_task_result, PersistedTaskRun, TaskRunStatus,
};
```

In `commands/mod.rs`, add:

```rust
pub fn list_task_runs_command(
    context: &CommandContext,
) -> Result<CommandEnvelope<Vec<PersistedTaskRun>>, DadrError> {
    let runs = crate::tasks::list_task_runs(&context.cwd)?;
    Ok(CommandEnvelope::new("task runs", runs))
}

pub fn show_task_result_command(
    context: &CommandContext,
    run_id: &str,
) -> Result<CommandEnvelope<serde_json::Value>, DadrError> {
    let runs = crate::tasks::list_task_runs(&context.cwd)?;
    let run = runs
        .into_iter()
        .find(|run| run.id == run_id)
        .ok_or_else(|| DadrError::InvalidInput(format!("task run `{run_id}` was not found")))?;
    let result = crate::tasks::read_task_result(&run.run_dir)?;
    Ok(CommandEnvelope::new("task result", result))
}
```

- [ ] **Step 6: Run CLI tests**

Run:

```bash
cargo test -p dadrock parses_task
cargo build -p dadrock
```

Expected: PASS and build succeeds.

- [ ] **Step 7: Commit Task 6**

```bash
git add crates/dadrock/src/cli.rs crates/dadrock/src/main.rs crates/dadrock-core/src/tasks/runs.rs crates/dadrock-core/src/commands/mod.rs
git commit -m "feat: add task CLI surface"
```

---

### Task 7: MCP Task Tools

**Files:**
- Modify: `crates/dadrock-core/src/mcp.rs`
- Modify: `crates/dadrock/tests/mcp_stdio.rs`

- [ ] **Step 1: Write failing MCP tool test**

In `crates/dadrock/tests/mcp_stdio.rs`, add this test:

```rust
#[tokio::test(flavor = "current_thread")]
async fn mcp_lists_task_tools() {
    let root = temp_project("dadrock-mcp-task-tools");
    init_project(&root).expect("initialize project");

    let mut command = Command::new(env!("CARGO_BIN_EXE_dadrock"));
    command.arg("mcp").arg("serve").arg(&root);
    let transport = TokioChildProcess::new(command).expect("spawn dadrock mcp serve");
    let mut client = ().serve(transport).await.expect("initialize MCP client");

    let tools = client.peer().list_all_tools().await.expect("list tools");
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();

    assert!(names.contains(&"dadrock.list_tasks"));
    assert!(names.contains(&"dadrock.get_task"));
    assert!(names.contains(&"dadrock.list_task_runs"));
    assert!(names.contains(&"dadrock.get_task_run"));

    let _ = tokio::time::timeout(Duration::from_secs(2), client.close()).await;
    let _ = fs::remove_dir_all(root);
}
```

- [ ] **Step 2: Run MCP tests to verify failure**

Run:

```bash
cargo test -p dadrock mcp_stdio
```

Expected: FAIL because MCP task tools are absent.

- [ ] **Step 3: Add MCP task tool constants**

Modify `crates/dadrock-core/src/mcp.rs`. Add constants near the other tool constants:

```rust
const TOOL_LIST_TASKS: &str = "dadrock.list_tasks";
const TOOL_GET_TASK: &str = "dadrock.get_task";
const TOOL_LIST_TASK_RUNS: &str = "dadrock.list_task_runs";
const TOOL_GET_TASK_RUN: &str = "dadrock.get_task_run";
```

- [ ] **Step 4: Add tools to `DadrockMcpServer::tools`**

Add read tools:

```rust
read_tool(
    TOOL_LIST_TASKS,
    "List built-in, project, and user Dadrock tasks.",
    empty_schema(),
),
read_tool(
    TOOL_GET_TASK,
    "Return one Dadrock task manifest and prompt.",
    object_schema(json!({
        "type": "object",
        "properties": {
            "task_id": {"type": "string"}
        },
        "required": ["task_id"],
        "additionalProperties": false
    })),
),
read_tool(
    TOOL_LIST_TASK_RUNS,
    "List persisted local Dadrock task runs.",
    empty_schema(),
),
read_tool(
    TOOL_GET_TASK_RUN,
    "Return one persisted Dadrock task run result.",
    object_schema(json!({
        "type": "object",
        "properties": {
            "run_id": {"type": "string"}
        },
        "required": ["run_id"],
        "additionalProperties": false
    })),
),
```

- [ ] **Step 5: Wire MCP handlers**

In `call_tool_value`, add match arms:

```rust
TOOL_LIST_TASKS => serde_json::to_value(commands::list_tasks_command(&self.context)?).map_err(Into::into),
TOOL_GET_TASK => {
    let task_id = required_string(&arguments, "task_id")?;
    serde_json::to_value(commands::show_task_command(&self.context, &task_id)?).map_err(Into::into)
}
TOOL_LIST_TASK_RUNS => serde_json::to_value(commands::list_task_runs_command(&self.context)?).map_err(Into::into),
TOOL_GET_TASK_RUN => {
    let run_id = required_string(&arguments, "run_id")?;
    serde_json::to_value(commands::show_task_result_command(&self.context, &run_id)?).map_err(Into::into)
}
```

- [ ] **Step 6: Update MCP annotation test**

In `mcp.rs` tests, assert that `dadrock.list_tasks` is read-only:

```rust
let list_tasks = tools
    .iter()
    .find(|tool| tool.name == "dadrock.list_tasks")
    .expect("list tasks tool");
assert!(list_tasks
    .annotations
    .as_ref()
    .and_then(|annotations| annotations.read_only_hint)
    .unwrap_or(false));
```

- [ ] **Step 7: Run MCP tests**

Run:

```bash
cargo test -p dadrock-core mcp
cargo test -p dadrock mcp_stdio
```

Expected: PASS.

- [ ] **Step 8: Commit Task 7**

```bash
git add crates/dadrock-core/src/mcp.rs crates/dadrock/tests/mcp_stdio.rs
git commit -m "feat: expose task tools over MCP"
```

---

### Task 8: Web API and Task Screen

**Files:**
- Modify: `crates/dadrock/src/web.rs`
- Modify: `crates/dadrock/src/web/static/index.html`
- Modify: `crates/dadrock/src/web/static/app.js`
- Modify: `crates/dadrock/src/web/static/app.css`

- [ ] **Step 1: Add web routes**

Modify `crates/dadrock/src/web.rs`. In `route_request`, add:

```rust
("GET", "/api/tasks") => list_tasks(cwd),
("GET", "/api/task-runs") => list_task_runs(cwd),
("POST", "/api/tasks/copy") => copy_task(request, cwd),
("POST", "/api/tasks/run") => run_task(request, cwd),
```

Before the fallback `GET` record handler, add dynamic GET handling:

```rust
("GET", task_path) => {
    if let Some(task_id) = task_query(task_path, "/api/tasks/") {
        return show_task(cwd, task_id);
    }
    if let Some(run_id) = task_query(task_path, "/api/task-runs/") {
        return show_task_run(cwd, run_id);
    }
    if let Some(query) = record_source_query(task_path) {
        return record_source(cwd, query);
    }
    json_response(404, &ApiError::not_found())
}
```

Add helper:

```rust
fn task_query<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    path.strip_prefix(prefix).filter(|value| !value.is_empty())
}
```

Add route handlers:

```rust
fn list_tasks(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::list_tasks_command(&context)?;
    json_response(200, &envelope)
}

fn show_task(cwd: &Utf8PathBuf, task_id: &str) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::show_task_command(&context, task_id)?;
    json_response(200, &envelope)
}

fn list_task_runs(cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::list_task_runs_command(&context)?;
    json_response(200, &envelope)
}

fn show_task_run(cwd: &Utf8PathBuf, run_id: &str) -> Result<HttpResponse, ApiFailure> {
    let context = context_for(cwd)?;
    let envelope = core_commands::show_task_result_command(&context, run_id)?;
    json_response(200, &envelope)
}
```

For `copy_task` and `run_task`, mirror the existing JSON route style:

```rust
#[derive(Debug, Deserialize)]
struct CopyTaskRequest {
    task_id: String,
}

#[derive(Debug, Deserialize)]
struct RunTaskRequest {
    task_id: String,
    #[serde(default)]
    input: serde_json::Value,
}

fn copy_task(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: CopyTaskRequest = json_body(request)?;
    let context = context_for(cwd)?;
    let envelope = core_commands::copy_task_command(&context, &body.task_id, None)?;
    json_response(200, &envelope)
}

fn run_task(request: &HttpRequest, cwd: &Utf8PathBuf) -> Result<HttpResponse, ApiFailure> {
    let body: RunTaskRequest = json_body(request)?;
    let local_agent = configured_local_acp_agent()?;
    let context = context_for(cwd)?;
    let envelope = core_commands::run_task_command(
        &context,
        dadrock_core::tasks::TaskRunCommandRequest {
            task_id: body.task_id,
            input: body.input,
            local_agent,
            permission_policy: AcpPermissionPolicy::AllowOnce,
            enable_dadrock_mcp: true,
        },
    )?;
    json_response(200, &envelope)
}
```

- [ ] **Step 2: Add Tasks nav tab**

Modify `crates/dadrock/src/web/static/index.html`:

```html
<button class="nav-tab" type="button" data-view="tasks">Tasks</button>
```

Place it after Decisions.

- [ ] **Step 3: Extend JS state**

Modify the initial `state` in `app.js`:

```javascript
tasks: null,
taskRuns: null,
selectedTaskId: null,
selectedRunId: null,
taskInput: {},
taskResult: null,
```

Update the nav click handler so when `state.view = "tasks"` it calls `loadTasks()` and `loadTaskRuns()`.

Add:

```javascript
async function loadTasks() {
  return request(
    "/api/tasks",
    { method: "GET" },
    (data) => {
      state.tasks = data.data;
      if (!state.selectedTaskId && state.tasks.tasks.length) {
        state.selectedTaskId = state.tasks.tasks[0].manifest.id;
      }
    },
    { silent: true }
  );
}

async function loadTaskRuns() {
  return request(
    "/api/task-runs",
    { method: "GET" },
    (data) => {
      state.taskRuns = data.data;
    },
    { silent: true }
  );
}
```

- [ ] **Step 4: Render Tasks screen**

In the main `render()` dispatcher, add:

```javascript
if (state.view === "tasks") {
  renderTasks();
  return;
}
```

Add:

```javascript
function renderTasks() {
  const tasks = state.tasks?.tasks || [];
  const runs = state.taskRuns || [];
  const selected = tasks.find((task) => task.manifest.id === state.selectedTaskId) || tasks[0];
  content.innerHTML = `
    <section class="tasks-grid">
      <aside class="task-library">
        <h2>Tasks</h2>
        ${tasks.map(renderTaskRow).join("") || `<p class="empty">No tasks found.</p>`}
      </aside>
      <section class="task-runner">
        ${selected ? renderTaskRunner(selected) : ""}
      </section>
      <aside class="task-history">
        <h2>Runs</h2>
        ${runs.map(renderTaskRunRow).join("") || `<p class="empty">No task runs yet.</p>`}
      </aside>
    </section>
    <section class="task-result-region">
      ${renderTaskResult()}
    </section>
  `;
  bindTaskEvents();
}

function renderTaskRow(task) {
  const active = task.manifest.id === state.selectedTaskId ? "active" : "";
  return `
    <button class="task-row ${active}" type="button" data-task-id="${escapeAttr(task.manifest.id)}">
      <span>${escapeHtml(task.manifest.title)}</span>
      <small>${escapeHtml(task.source.kind)} · ${escapeHtml(task.manifest.output.kind)}</small>
    </button>
  `;
}

function renderTaskRunner(task) {
  const copyButton = task.source.kind === "project"
    ? ""
    : `<button class="button" type="button" data-copy-task="${escapeAttr(task.manifest.id)}">Copy to project</button>`;
  return `
    <header class="pane-header">
      <div>
        <h2>${escapeHtml(task.manifest.title)}</h2>
        <p>${escapeHtml(task.manifest.description)}</p>
      </div>
      ${copyButton}
      <button class="button primary" type="button" data-run-task="${escapeAttr(task.manifest.id)}">Run</button>
    </header>
    <form class="task-form">
      ${(task.manifest.inputs || []).map(renderTaskInput).join("")}
    </form>
    <details class="prompt-preview">
      <summary>Prompt</summary>
      <pre>${escapeHtml(task.prompt || "")}</pre>
    </details>
  `;
}

function renderTaskInput(input) {
  const value = state.taskInput[input.name] ?? input.default ?? "";
  if (input.type === "textarea") {
    return `<label><span>${escapeHtml(input.label)}</span><textarea data-task-input="${escapeAttr(input.name)}">${escapeHtml(value)}</textarea></label>`;
  }
  if (input.type === "select") {
    return `<label><span>${escapeHtml(input.label)}</span><select data-task-input="${escapeAttr(input.name)}">${(input.options || []).map((option) => `<option value="${escapeAttr(option)}" ${option === value ? "selected" : ""}>${escapeHtml(option)}</option>`).join("")}</select></label>`;
  }
  if (input.type === "checkbox") {
    return `<label class="checkbox-line"><input type="checkbox" data-task-input="${escapeAttr(input.name)}" ${value === true || value === "true" ? "checked" : ""}><span>${escapeHtml(input.label)}</span></label>`;
  }
  return `<label><span>${escapeHtml(input.label)}</span><input data-task-input="${escapeAttr(input.name)}" value="${escapeAttr(value)}"></label>`;
}

function renderTaskRunRow(run) {
  const active = run.id === state.selectedRunId ? "active" : "";
  return `
    <button class="task-run-row ${active}" type="button" data-run-id="${escapeAttr(run.id)}">
      <span>${escapeHtml(run.task_id)}</span>
      <small>${escapeHtml(run.status)} · ${escapeHtml(run.created_at)}</small>
    </button>
  `;
}
```

- [ ] **Step 5: Bind task events**

Add:

```javascript
function bindTaskEvents() {
  content.querySelectorAll("[data-task-id]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedTaskId = button.dataset.taskId;
      state.taskInput = {};
      state.taskResult = null;
      renderTasks();
    });
  });
  content.querySelectorAll("[data-task-input]").forEach((input) => {
    input.addEventListener("input", () => {
      state.taskInput[input.dataset.taskInput] =
        input.type === "checkbox" ? input.checked : input.value;
    });
  });
  content.querySelectorAll("[data-run-task]").forEach((button) => {
    button.addEventListener("click", () => runSelectedTask(button.dataset.runTask));
  });
  content.querySelectorAll("[data-copy-task]").forEach((button) => {
    button.addEventListener("click", () => copyTaskToProject(button.dataset.copyTask));
  });
  content.querySelectorAll("[data-run-id]").forEach((button) => {
    button.addEventListener("click", () => loadTaskResult(button.dataset.runId));
  });
}

async function copyTaskToProject(taskId) {
  await request(
    "/api/tasks/copy",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ task_id: taskId })
    },
    () => {
      pushLog(`copied task ${taskId} to project`);
    }
  );
  await loadTasks();
}

async function runSelectedTask(taskId) {
  await request(
    "/api/tasks/run",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ task_id: taskId, input: state.taskInput })
    },
    (data) => {
      pushLog(`task run ${data.data.id} succeeded`);
      state.selectedRunId = data.data.id;
    }
  );
  await loadTaskRuns();
}

async function loadTaskResult(runId) {
  state.selectedRunId = runId;
  return request(
    `/api/task-runs/${encodeURIComponent(runId)}`,
    { method: "GET" },
    (data) => {
      state.taskResult = data.data;
    }
  );
}
```

- [ ] **Step 6: Add result renderers**

Add:

```javascript
function renderTaskResult() {
  const result = state.taskResult;
  if (!result) return "";
  const kind = result.kind;
  if (kind === "candidate_draft.v1") return renderCandidateDraftResult(result);
  if (kind === "candidate_batch.v1") return renderCandidateBatchResult(result);
  if (kind === "conflict_set.v1") return renderConflictSetResult(result);
  if (kind === "diagnosis_report.v1") return renderDiagnosisReportResult(result);
  return renderGenericTaskResult(result);
}

function renderCandidateDraftResult(result) {
  return `
    <article class="task-result task-result-single">
      <header><h2>${escapeHtml(result.title)}</h2></header>
      <textarea class="task-markdown-editor" data-task-draft-body>${escapeHtml(result.body_markdown || "")}</textarea>
      <button class="button primary" type="button" data-write-task-candidate>Write candidate</button>
    </article>
  `;
}

function renderCandidateBatchResult(result) {
  return `
    <article class="task-result">
      <h2>${escapeHtml(result.summary || "Candidate proposals")}</h2>
      ${(result.candidates || []).map((candidate) => `
        <section class="proposal-row">
          <h3>${escapeHtml(candidate.title)}</h3>
          <p>${escapeHtml(candidate.confidence || "unknown")}</p>
          <pre>${escapeHtml(candidate.body_markdown || "")}</pre>
        </section>
      `).join("")}
    </article>
  `;
}

function renderConflictSetResult(result) {
  return `
    <article class="task-result">
      <h2>${escapeHtml(result.summary || "Conflict findings")}</h2>
      ${(result.findings || []).map((finding) => `
        <section class="conflict-finding">
          <header>
            <h3>${escapeHtml(finding.title)}</h3>
            <small>${escapeHtml(finding.relationship)}</small>
          </header>
          <div class="conflict-grid">
            <div class="conflict-record">${escapeHtml(finding.left_record_id)}</div>
            <div class="conflict-explanation">
              <strong>Conflict</strong>
              <p>${escapeHtml(finding.conflict_explanation)}</p>
              <strong>Why this might not be a conflict</strong>
              <p>${escapeHtml(finding.why_might_not_be_conflict)}</p>
            </div>
            <div class="conflict-record">${escapeHtml(finding.right_record_id)}</div>
          </div>
        </section>
      `).join("")}
    </article>
  `;
}

function renderDiagnosisReportResult(result) {
  return `
    <article class="task-result">
      <h2>${escapeHtml(result.summary || "Diagnosis")}</h2>
      ${(result.issues || []).map((issue) => `
        <section class="issue-row">
          <strong>${escapeHtml(issue.title)}</strong>
          <span>${escapeHtml(issue.severity)} · ${escapeHtml(issue.category)}</span>
          <p>${escapeHtml(issue.explanation)}</p>
        </section>
      `).join("")}
    </article>
  `;
}

function renderGenericTaskResult(result) {
  return `
    <article class="task-result">
      <h2>${escapeHtml(result.summary || result.kind || "Task result")}</h2>
      ${result.markdown ? renderMarkdown(result.markdown) : ""}
      <details open>
        <summary>Raw JSON</summary>
        <pre>${escapeHtml(JSON.stringify(result, null, 2))}</pre>
      </details>
    </article>
  `;
}
```

Add the write action binding after renderers are in place:

```javascript
content.querySelectorAll("[data-write-task-candidate]").forEach((button) => {
  button.addEventListener("click", async () => {
    const result = state.taskResult;
    if (!result || result.kind !== "candidate_draft.v1") return;
    await postJson(
      "/api/records",
      {
        title: result.title,
        tags: result.tags || [],
        body_markdown: content.querySelector("[data-task-draft-body]").value
      },
      "candidate created"
    );
  });
});
```

- [ ] **Step 7: Add CSS**

Add to `app.css`:

```css
.tasks-grid {
  display: grid;
  grid-template-columns: minmax(220px, 280px) minmax(0, 1fr) minmax(220px, 280px);
  gap: 12px;
  min-height: 360px;
}

.task-library,
.task-runner,
.task-history,
.task-result {
  min-width: 0;
}

.task-row,
.task-run-row {
  width: 100%;
  display: grid;
  gap: 4px;
  text-align: left;
  margin-bottom: 6px;
}

.task-row.active,
.task-run-row.active {
  border-color: var(--accent);
}

.task-form {
  display: grid;
  gap: 12px;
}

.task-result-region {
  margin-top: 12px;
}

.task-markdown-editor {
  width: 100%;
  min-height: 280px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
}

.conflict-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(260px, 0.8fr) minmax(0, 1fr);
  gap: 12px;
}

.conflict-record,
.conflict-explanation,
.issue-row,
.proposal-row {
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 12px;
}

@media (max-width: 920px) {
  .tasks-grid,
  .conflict-grid {
    grid-template-columns: 1fr;
  }
}
```

- [ ] **Step 8: Verify web assets**

Run:

```bash
node --check crates/dadrock/src/web/static/app.js
cargo build -p dadrock
```

Expected: PASS and build succeeds.

- [ ] **Step 9: Commit Task 8**

```bash
git add crates/dadrock/src/web.rs crates/dadrock/src/web/static/index.html crates/dadrock/src/web/static/app.js crates/dadrock/src/web/static/app.css
git commit -m "feat: add task screen to web preview"
```

---

### Task 9: Docs and Final Verification

**Files:**
- Modify: `docs-src/dadrock/cli.md`
- Modify: `docs-src/dadrock/web.md`
- Modify: `docs-src/llms.md`

- [ ] **Step 1: Update CLI docs**

Regenerate CLI docs with the existing hidden command:

```bash
cargo run -p dadrock -- gen-cli-docs > docs-src/dadrock/cli.md
```

Expected: `docs-src/dadrock/cli.md` includes `dadrock task`, `task list`, `task show`, `task run`, `task runs`, and `task result`.

- [ ] **Step 2: Update web docs**

Add a concise Tasks section to `docs-src/dadrock/web.md`:

```markdown
## Tasks

The preview UI includes a Tasks tab for agent-assisted workflows over the
decision ledger. Tasks can be built into Dadrock, defined by the project under
`.dadrock/tasks/`, or installed in a user task directory. Task runs are saved
under `.dadrock/runs/`, which Dadrock ignores by default because runs may
contain pasted context, source excerpts, and model transcripts.

Task results use typed output schemas. Candidate drafts open in a one-panel
editor, conflict findings open in a side-by-side review, diagnosis reports open
as issue lists, and unknown third-party result kinds fall back to a report plus
raw JSON view.
```

- [ ] **Step 3: Update LLM docs**

Add to `docs-src/llms.md`:

```markdown
## Dadrock Tasks

Dadrock tasks are editable prompt workflows that run through the same Dadrock
MCP and command layers as the CLI and web UI. Built-in tasks are examples, not
special cases. Project tasks live under `.dadrock/tasks/` and task runs live
under `.dadrock/runs/`.

Agents running tasks must return the declared JSON output kind exactly. They
must not accept, reject, supersede, or rewrite records except through explicit
Dadrock lifecycle actions requested by the user.
```

- [ ] **Step 4: Run targeted tests**

Run:

```bash
cargo test -p dadrock-core task_system
cargo test -p dadrock-core commands
cargo test -p dadrock mcp_stdio
node --check crates/dadrock/src/web/static/app.js
```

Expected: all commands pass.

- [ ] **Step 5: Run full test suite**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 6: Review git diff**

Run:

```bash
git diff --stat
git diff --check
```

Expected: no whitespace errors. The diff should be limited to task-system code, web UI task rendering, and docs.

- [ ] **Step 7: Commit docs and final polish**

```bash
git add docs-src/dadrock/cli.md docs-src/dadrock/web.md docs-src/llms.md
git commit -m "docs: describe Dadrock tasks"
```

---

## Self-Review Notes

Spec coverage:

- Task locations and same-format built-ins: Tasks 2 and 4.
- User task discovery and project shadowing: Tasks 2 and 4.
- Copy-to-project customization: Tasks 4, 6, and 8.
- Persisted gitignored runs: Task 3.
- Typed output schemas and fallback vendor kinds: Tasks 1 and 5.
- Raw JSON ACP task execution: Task 5.
- Web task library, runner, history, and renderers: Task 8.
- Built-in starter task mapping: Task 2.
- Core command/MCP/CLI API surfaces: Tasks 4, 6, and 7.
- Error handling and invalid diagnostics: Tasks 2, 3, 5, and 8.
- Tests: every task includes targeted tests; Task 9 runs final verification.

Implementation order keeps each checkpoint buildable before moving outward from core to UI.
