use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

pub use dadr::{
    check_required_sections, missing_required_sections, parse_record_str, ParsedRecord,
    SectionCheck,
};

use crate::{
    error::{DadrError, Diagnostic, DiagnosticCode},
    file_store,
    ids::RecordId,
    index::ProjectIndex,
    markdown::frontmatter::split_front_matter,
    model::{Record, RecordStatus},
    project::{utf8_to_path, ProjectContext},
    templates::candidate_template_body,
    text::ascii_slug,
};

mod frozen;
mod supersession;

pub use frozen::check_frozen_rewrite;
pub(crate) use supersession::record_list;
pub use supersession::{
    accept_candidate, accept_candidates, current_successors, pending_supersedes,
    successor_candidate_body, PendingSupersedes, StaleSupersedes,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannedRecord {
    pub path: Utf8PathBuf,
    pub record: Option<Record>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ScannedRecord {
    pub fn is_valid(&self) -> bool {
        self.record.is_some() && !self.diagnostics.iter().any(Diagnostic::is_error)
    }

    pub fn summary(&self) -> Option<ScannedRecordSummary> {
        let record = self.record.as_ref()?;
        Some(ScannedRecordSummary {
            path: self.path.clone(),
            id: record.front_matter.id,
            title: record.front_matter.title.clone(),
            status: record.front_matter.status,
            superseded: record.front_matter.is_superseded(),
            timestamp_ms: record.front_matter.id.timestamp_millis(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScannedRecordSummary {
    pub path: Utf8PathBuf,
    pub id: RecordId,
    pub title: String,
    pub status: RecordStatus,
    #[serde(default)]
    pub superseded: bool,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindResolution {
    None,
    One,
    Many,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    ExactId,
    IdPrefix,
    Filename,
    Title,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundRecord {
    pub match_kind: MatchKind,
    pub record: ScannedRecordSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindRecordsResult {
    pub query: String,
    pub resolution: FindResolution,
    pub matches: Vec<FoundRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewRecordResult {
    pub id: RecordId,
    pub path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCandidateResult {
    pub id: RecordId,
    pub path: Utf8PathBuf,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRevisionPreview {
    pub id: RecordId,
    pub path: Utf8PathBuf,
    pub title: String,
    pub current_content: String,
    pub proposed_content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordStatusResult {
    pub record: ScannedRecordSummary,
    pub successors: Vec<ScannedRecordSummary>,
    pub predecessors: Vec<ScannedRecordSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectRecordResult {
    pub id: RecordId,
    pub from: Utf8PathBuf,
    pub to: Utf8PathBuf,
    pub status: RecordStatus,
}

/// A record file rewritten by a lifecycle action, with where it was and where
/// it is now (the same path when it did not change folders).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordMove {
    pub id: RecordId,
    pub from: Utf8PathBuf,
    pub to: Utf8PathBuf,
}

/// The result of accepting a set of candidates in one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptRecordResult {
    /// Each accepted candidate's move from the inbox to the ledger, in the
    /// order the candidates were given.
    pub accepted: Vec<RecordMove>,
    /// Each predecessor named in a `## Supersedes` section of the set, moved
    /// to the archive.
    #[serde(default)]
    pub superseded: Vec<SupersededRecordMove>,
}

/// A predecessor superseded by accepting its successors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersededRecordMove {
    pub id: RecordId,
    pub from: Utf8PathBuf,
    pub to: Utf8PathBuf,
    /// The accepted records added to its `successors`.
    pub successors: Vec<RecordId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersedeRecordResult {
    pub old_id: RecordId,
    pub new_id: RecordId,
    pub old_path: Utf8PathBuf,
    pub new_path: Utf8PathBuf,
    pub status: RecordStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersedeRecordsResult {
    /// The superseded records, as they are after the operation.
    pub from: Vec<ScannedRecordSummary>,
    pub to: Vec<ScannedRecordSummary>,
    pub edges: Vec<SupersedeRecordResult>,
    /// Each superseded record's move to the archive.
    #[serde(default)]
    pub superseded: Vec<RecordMove>,
}

pub fn parse_record_file(path: &Utf8Path) -> Result<ScannedRecord, DadrError> {
    let content = match fs::read_to_string(utf8_to_path(path)) {
        Ok(content) => content,
        Err(err) => {
            return Ok(ScannedRecord {
                path: path.to_owned(),
                record: None,
                diagnostics: vec![Diagnostic::error(
                    DiagnosticCode::UnreadableRecord,
                    format!("record file could not be read as UTF-8 text: {err}"),
                )
                .with_path(path.to_owned())],
            });
        }
    };
    let mut parsed = parse_record_str(&content);
    parsed.diagnostics = parsed
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.with_path(path.to_owned()))
        .collect();
    Ok(ScannedRecord {
        path: path.to_owned(),
        record: parsed.record,
        diagnostics: parsed.diagnostics,
    })
}

pub fn scan_records(dir: &Utf8Path) -> Result<Vec<ScannedRecord>, DadrError> {
    let mut records = Vec::new();
    if !dir.is_dir() {
        return Ok(records);
    }
    scan_dir(dir, &mut records)?;
    records.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(records)
}

pub fn scan_input(path: &Utf8Path) -> Result<Vec<ScannedRecord>, DadrError> {
    if path.is_dir() {
        scan_records(path)
    } else {
        Ok(vec![parse_record_file(path)?])
    }
}

pub fn status_record(index: &ProjectIndex, query: &str) -> Result<RecordStatusResult, DadrError> {
    let found = index.resolve_one(query)?;
    let scanned = scanned_record_for_summary(index, &found.record)?;

    let successors = scanned
        .record
        .as_ref()
        .map(|record| record.front_matter.successors.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| index.summary_by_id(id).cloned())
        .collect();

    let predecessors = index
        .scanned_records()
        .iter()
        .filter_map(|candidate| {
            let record = candidate.record.as_ref()?;
            if record.front_matter.successors.contains(&found.record.id) {
                candidate.summary()
            } else {
                None
            }
        })
        .collect();

    Ok(RecordStatusResult {
        record: found.record,
        successors,
        predecessors,
        diagnostics: scanned.diagnostics.clone(),
    })
}

pub fn create_candidate(
    project: &ProjectContext,
    title: &str,
    tags: &[String],
) -> Result<NewRecordResult, DadrError> {
    let title = validated_candidate_title(title)?;
    let template = candidate_template_body(project)?;
    create_candidate_from_body(project, title, tags, &template.body)
}

pub fn create_candidate_with_body(
    project: &ProjectContext,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<NewRecordResult, DadrError> {
    let title = validated_candidate_title(title)?;
    if body_markdown.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "candidate body cannot be empty".to_owned(),
        ));
    }

    create_candidate_from_body(project, title, tags, body_markdown)
}

fn validated_candidate_title(title: &str) -> Result<&str, DadrError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(DadrError::InvalidInput(
            "record title cannot be empty".to_owned(),
        ));
    }
    Ok(title)
}

fn create_candidate_from_body(
    project: &ProjectContext,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<NewRecordResult, DadrError> {
    project.require_initialized()?;
    let index = ProjectIndex::scan(project)?;
    let id = allocate_record_id(&index);
    let inbox_dir = project.inbox_dir();
    fs::create_dir_all(utf8_to_path(&inbox_dir))?;
    let filename = record_filename(id, title);
    let path = inbox_dir.join(filename);
    let content = render_candidate(id, title, tags, body_markdown);

    file_store::write_new(&path, &content)?;

    Ok(NewRecordResult { id, path })
}

pub fn update_candidate_with_body(
    index: &ProjectIndex,
    query: &str,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<UpdateCandidateResult, DadrError> {
    let preview = preview_candidate_revision(index, query, title, tags, body_markdown)?;
    file_store::write_replace(&preview.path, &preview.proposed_content)?;

    Ok(UpdateCandidateResult {
        id: preview.id,
        path: preview.path,
        title: preview.title,
    })
}

pub fn preview_candidate_revision(
    index: &ProjectIndex,
    query: &str,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<CandidateRevisionPreview, DadrError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(DadrError::InvalidInput(
            "record title cannot be empty".to_owned(),
        ));
    }
    if body_markdown.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "candidate body cannot be empty".to_owned(),
        ));
    }

    let found = index.resolve_one(query)?;
    let record = require_record_status(
        index,
        &found.record,
        RecordStatus::Candidate,
        "revised",
        "revise requires status `candidate`",
    )?;

    let original_content = fs::read_to_string(utf8_to_path(&found.record.path))?;
    let revised_content = rewrite_candidate_content(&original_content, title, tags, body_markdown)?;

    Ok(CandidateRevisionPreview {
        id: record.front_matter.id,
        path: found.record.path,
        title: title.to_owned(),
        current_content: original_content,
        proposed_content: revised_content,
    })
}

pub fn reject_candidate(
    index: &ProjectIndex,
    query: &str,
) -> Result<RejectRecordResult, DadrError> {
    let project = index.project();
    let found = index.resolve_one(query)?;
    require_record_status(
        index,
        &found.record,
        RecordStatus::Candidate,
        "rejected",
        "reject requires status `candidate`",
    )?;

    let target_path = project
        .archive_dir()
        .join(record_file_name(&found.record.path)?);
    let write = RecordWrite::read(&found.record.path, target_path.clone(), |content| {
        rewrite_status(content, RecordStatus::Rejected)
    })?;
    apply_record_writes(std::slice::from_ref(&write))?;

    Ok(RejectRecordResult {
        id: found.record.id,
        from: found.record.path,
        to: target_path,
        status: RecordStatus::Rejected,
    })
}

/// The `## Supersedes` section of a record: the records it supersedes, by full
/// id, and optionally why, as Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Supersedes {
    pub ids: Vec<RecordId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

const SUPERSEDES_HEADING: &str = "## Supersedes";

/// Parse the optional `## Supersedes` section of a record body.
///
/// The section starts with one or more lines that are each exactly one record
/// link by full id, `[[uuid]]` or `[[uuid|label]]`. The first other non-empty
/// line starts the reason, which runs to the end of the section and keeps its
/// Markdown. Headings inside fenced or indented code neither open nor close the
/// section, and code inside the section is never a link line.
pub fn supersedes_section(body: &str) -> Result<Option<Supersedes>, DadrError> {
    let lines = classify_body_lines(body);
    let section = match supersedes_sections(&lines).as_slice() {
        [] => return Ok(None),
        [section] => section.clone(),
        _ => {
            return Err(DadrError::InvalidInput(
                "a record can have only one `## Supersedes` section",
            ))
        }
    };

    // Skip the heading; the links come first, then the reason.
    let content = &lines[section.start + 1..section.end];
    let mut ids = Vec::new();
    let mut reason_lines: &[BodyLine<'_>] = &[];
    for (offset, line) in content.iter().enumerate() {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        let link = (line.kind == LineKind::Text)
            .then(|| single_wikilink(text))
            .flatten();
        let Some(link) = link else {
            reason_lines = &content[offset..];
            break;
        };
        let id = supersedes_link_id(link)?;
        if ids.contains(&id) {
            return Err(DadrError::InvalidInput(format!(
                "`## Supersedes` names `{id}` more than once"
            )));
        }
        ids.push(id);
    }
    if ids.is_empty() {
        return Err(DadrError::InvalidInput(
            "`## Supersedes` must start with a link to the superseded record by full id, such as [[<uuid>|<title>]]",
        ));
    }

    let mut reason: Vec<&str> = reason_lines.iter().map(|line| line.text).collect();
    while reason.last().is_some_and(|line| line.trim().is_empty()) {
        reason.pop();
    }
    let reason = (!reason.is_empty()).then(|| reason.join("\n"));
    Ok(Some(Supersedes { ids, reason }))
}

/// The body with every `## Supersedes` section removed, for comparing versions.
/// The rest of the body is unchanged, except that blank lines left at the end
/// by removing a final section are dropped.
pub fn body_without_supersedes(body: &str) -> String {
    let lines = classify_body_lines(body);
    let sections = supersedes_sections(&lines);
    if sections.is_empty() {
        return body.to_owned();
    }
    let mut output = String::with_capacity(body.len());
    let mut next = 0;
    for section in &sections {
        output.push_str(&body[next..lines[section.start].start]);
        next = lines.get(section.end).map_or(body.len(), |line| line.start);
    }
    output.push_str(&body[next..]);
    if next == body.len() {
        let kept = output.trim_end().len();
        output.truncate(kept);
        if !output.is_empty() {
            output.push('\n');
        }
    }
    output
}

/// Prefix a body with a `## Supersedes` section: one link per predecessor,
/// labelled with its title, then the reason, if any.
pub fn with_supersedes_section(
    body: &str,
    predecessors: &[(RecordId, &str)],
    reason: Option<&str>,
) -> String {
    let mut section = format!("{SUPERSEDES_HEADING}\n\n");
    for (id, title) in predecessors {
        // A label is one line and cannot contain the link delimiters.
        let label = title
            .replace(['\r', '\n'], " ")
            .replace("[[", "[")
            .replace("]]", "]");
        section.push_str(&format!("[[{id}|{}]]\n", label.trim()));
    }
    if let Some(reason) = reason.map(str::trim).filter(|reason| !reason.is_empty()) {
        section.push('\n');
        section.push_str(reason);
        section.push('\n');
    }
    section.push('\n');
    section.push_str(body.trim_start());
    section
}

/// How a body line reads for the `## Supersedes` grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind {
    /// Indented code, a code fence, or a line inside a fenced block.
    Code,
    /// A level-1 or level-2 ATX heading, which starts a new section.
    SectionHeading {
        supersedes: bool,
    },
    Text,
}

struct BodyLine<'a> {
    /// The line without its line ending.
    text: &'a str,
    /// Byte offset of the line in the body.
    start: usize,
    kind: LineKind,
}

fn classify_body_lines(body: &str) -> Vec<BodyLine<'_>> {
    let mut lines = Vec::new();
    let mut fence = None;
    let mut start = 0;
    for raw in body.split_inclusive('\n') {
        let text = raw.strip_suffix('\n').unwrap_or(raw);
        let text = text.strip_suffix('\r').unwrap_or(text);
        let kind = classify_body_line(text, &mut fence);
        lines.push(BodyLine { text, start, kind });
        start += raw.len();
    }
    lines
}

fn classify_body_line(text: &str, fence: &mut Option<(char, usize)>) -> LineKind {
    // Indented code cannot open or close a fence or a section.
    if text.starts_with("    ") || text.starts_with('\t') {
        return LineKind::Code;
    }
    let line = text.trim_end();
    let fence_line = line.trim_start();
    let marker = fence_line
        .chars()
        .next()
        .filter(|ch| *ch == '`' || *ch == '~');
    let count = marker
        .map(|ch| fence_line.chars().take_while(|c| *c == ch).count())
        .unwrap_or(0);
    if let Some((open_marker, open_count)) = *fence {
        if marker == Some(open_marker)
            && count >= open_count
            && fence_line[count..].trim().is_empty()
        {
            *fence = None;
        }
        return LineKind::Code;
    }
    if count >= 3 {
        *fence = marker.map(|ch| (ch, count));
        return LineKind::Code;
    }
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=2).contains(&level) && (line.len() == level || line[level..].starts_with([' ', '\t'])) {
        return LineKind::SectionHeading {
            supersedes: line == SUPERSEDES_HEADING,
        };
    }
    LineKind::Text
}

/// Line ranges of each `## Supersedes` section: the heading through the line
/// before the next level-1 or level-2 heading.
fn supersedes_sections(lines: &[BodyLine<'_>]) -> Vec<std::ops::Range<usize>> {
    let mut sections = Vec::new();
    let mut open = None;
    for (index, line) in lines.iter().enumerate() {
        if let LineKind::SectionHeading { supersedes } = line.kind {
            if let Some(start) = open.take() {
                sections.push(start..index);
            }
            if supersedes {
                open = Some(index);
            }
        }
    }
    if let Some(start) = open {
        sections.push(start..lines.len());
    }
    sections
}

/// The inside of a line that is exactly one wikilink.
fn single_wikilink(text: &str) -> Option<&str> {
    let inner = text.strip_prefix("[[")?.strip_suffix("]]")?;
    (!inner.contains("[[") && !inner.contains("]]")).then_some(inner)
}

fn supersedes_link_id(link: &str) -> Result<RecordId, DadrError> {
    let target = link.split('|').next().unwrap_or_default().trim();
    RecordId::parse(target).map_err(|_| {
        DadrError::InvalidInput(format!(
            "`## Supersedes` link `[[{link}]]` must name a record by its full id, such as [[<uuid>|<title>]]"
        ))
    })
}

pub fn supersede_record(
    index: &ProjectIndex,
    old_query: &str,
    new_query: &str,
) -> Result<SupersedeRecordResult, DadrError> {
    let result = supersede_records(index, &[old_query.to_owned()], &[new_query.to_owned()])?;
    result
        .edges
        .into_iter()
        .next()
        .ok_or_else(|| DadrError::ProjectInvariant("supersede produced no edge".to_owned()))
}

pub fn supersede_records(
    index: &ProjectIndex,
    old_queries: &[String],
    new_queries: &[String],
) -> Result<SupersedeRecordsResult, DadrError> {
    if old_queries.is_empty() {
        return Err(DadrError::InvalidInput(
            "supersede requires at least one --from query".to_owned(),
        ));
    }
    if new_queries.is_empty() {
        return Err(DadrError::InvalidInput(
            "supersede requires at least one --to query".to_owned(),
        ));
    }

    let mut old_records = resolve_unique_summaries(index, old_queries, "--from")?;
    let new_records = resolve_unique_summaries(index, new_queries, "--to")?;
    let new_ids: BTreeSet<_> = new_records.iter().map(|record| record.id).collect();
    for old in &old_records {
        if new_ids.contains(&old.id) {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "`{}` cannot appear in both --from and --to",
                old.id
            )));
        }
    }

    for old in &old_records {
        let old_record = supersedable_record(index, old)?;
        for new in &new_records {
            if old_record.front_matter.successors.contains(&new.id) {
                return Err(DadrError::InvalidLifecycleTransition(format!(
                    "`{}` already lists `{}` as a successor",
                    old.id, new.id
                )));
            }
        }
    }
    for new in &new_records {
        accepted_successor_record(index, new)?;
    }

    let mut graph = successor_graph(index.scanned_records());
    for old in &old_records {
        let successors = graph.entry(old.id).or_default();
        for new in &new_records {
            if !successors.contains(&new.id) {
                successors.push(new.id);
            }
        }
    }
    for old in &old_records {
        for new in &new_records {
            if reaches_record(&graph, new.id, old.id, &mut BTreeSet::new()) {
                return Err(DadrError::InvalidLifecycleTransition(format!(
                    "adding successor `{}` to `{}` would create a cycle",
                    new.id, old.id
                )));
            }
        }
    }

    // Every predecessor gains its successors and moves to the archive in one
    // operation; on failure, every record is restored.
    let new_ids: Vec<_> = new_records.iter().map(|record| record.id).collect();
    let archive_dir = index.project().archive_dir();
    let writes = old_records
        .iter()
        .map(|old| {
            let archived_path = archive_dir.join(record_file_name(&old.path)?);
            RecordWrite::read(&old.path, archived_path, |content| {
                rewrite_for_superseded_record(content, &new_ids)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    apply_record_writes(&writes)?;

    let mut superseded = Vec::new();
    for (old, write) in old_records.iter_mut().zip(&writes) {
        superseded.push(RecordMove {
            id: old.id,
            from: write.from.clone(),
            to: write.to.clone(),
        });
        old.path = write.to.clone();
        old.superseded = true;
    }

    let mut edges = Vec::new();
    for old in &old_records {
        for new in &new_records {
            edges.push(SupersedeRecordResult {
                old_id: old.id,
                new_id: new.id,
                old_path: old.path.clone(),
                new_path: new.path.clone(),
                status: RecordStatus::Accepted,
            });
        }
    }

    Ok(SupersedeRecordsResult {
        from: old_records,
        to: new_records,
        edges,
        superseded,
    })
}

/// One record file, rewritten in place or moved to another folder.
struct RecordWrite {
    from: Utf8PathBuf,
    to: Utf8PathBuf,
    original: String,
    updated: String,
}

impl RecordWrite {
    fn read(
        from: &Utf8Path,
        to: Utf8PathBuf,
        rewrite: impl FnOnce(&str) -> Result<String, DadrError>,
    ) -> Result<Self, DadrError> {
        let original = fs::read_to_string(utf8_to_path(from))?;
        let updated = rewrite(&original)?;
        Ok(Self {
            from: from.to_owned(),
            to,
            original,
            updated,
        })
    }

    fn moves(&self) -> bool {
        self.from != self.to
    }
}

/// Apply every write or none. All targets are staged before any is committed,
/// and the sources of moves are removed last. On an I/O error, the original
/// files come back and moved copies are removed.
fn apply_record_writes(writes: &[RecordWrite]) -> Result<(), DadrError> {
    let mut targets = BTreeSet::new();
    for write in writes {
        if !targets.insert(&write.to) {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "two records would be written to `{}`",
                write.to
            )));
        }
        if write.moves() && write.to.exists() {
            return Err(DadrError::OverwriteCollision {
                path: write.to.clone(),
            });
        }
    }

    let staged = writes
        .iter()
        .map(|write| file_store::stage_replace(&write.to, &write.updated))
        .collect::<Result<Vec<_>, _>>()?;
    let committed = (|| {
        for staged in staged {
            staged.commit()?;
        }
        for write in writes.iter().filter(|write| write.moves()) {
            file_store::remove_file(&write.from)?;
        }
        Ok::<_, DadrError>(())
    })();
    if let Err(error) = committed {
        restore_record_writes(writes);
        return Err(error);
    }
    Ok(())
}

fn restore_record_writes(writes: &[RecordWrite]) {
    // Bring every original back before removing any moved copy, so no record
    // is ever missing from disk.
    for write in writes {
        if !write.moves() || !write.from.exists() {
            let _ = file_store::write_replace(&write.from, &write.original);
        }
    }
    for write in writes.iter().filter(|write| write.moves()) {
        if write.to.exists() {
            let _ = file_store::remove_file(&write.to);
        }
    }
}

fn record_file_name(path: &Utf8Path) -> Result<&str, DadrError> {
    path.file_name()
        .ok_or_else(|| DadrError::ProjectInvariant(format!("record path `{path}` has no filename")))
}

fn scan_dir(dir: &Utf8Path, records: &mut Vec<ScannedRecord>) -> Result<(), DadrError> {
    for entry in fs::read_dir(utf8_to_path(dir))? {
        let entry = entry?;
        let path =
            Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| DadrError::PathNotUtf8 {
                path: path.display().to_string(),
            })?;

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if should_skip_dir(&file_name) {
                continue;
            }
            scan_dir(&path, records)?;
        } else if file_type.is_file() && path.extension() == Some("md") {
            records.push(parse_record_file(&path)?);
        }
    }
    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.') || matches!(name, "_reports" | "_site" | "templates")
}

fn scanned_record_for_summary<'a>(
    index: &'a ProjectIndex,
    summary: &ScannedRecordSummary,
) -> Result<&'a ScannedRecord, DadrError> {
    index
        .scanned_records()
        .iter()
        .find(|scanned| scanned.path == summary.path)
        .ok_or_else(|| {
            DadrError::ProjectInvariant(format!("indexed record `{}` is missing", summary.path))
        })
}

fn require_record_status<'a>(
    index: &'a ProjectIndex,
    summary: &ScannedRecordSummary,
    required: RecordStatus,
    invalid_verb: &str,
    status_message: &str,
) -> Result<&'a Record, DadrError> {
    let scanned = scanned_record_for_summary(index, summary)?;
    if !scanned.is_valid() {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` has validation errors and cannot be {invalid_verb}",
            summary.path
        )));
    }
    let record = scanned.record.as_ref().ok_or_else(|| {
        DadrError::ProjectInvariant(format!("failed to parse `{}`", summary.path))
    })?;
    if record.front_matter.status != required {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "{status_message}, found `{}`",
            record.front_matter.status
        )));
    }
    Ok(record)
}

fn resolve_unique_summaries(
    index: &ProjectIndex,
    queries: &[String],
    flag: &str,
) -> Result<Vec<ScannedRecordSummary>, DadrError> {
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for query in queries {
        let found = index.resolve_one(query)?;
        if !seen.insert(found.record.id) {
            return Err(DadrError::InvalidInput(format!(
                "duplicate {flag} query `{query}` resolved to `{}`",
                found.record.id
            )));
        }
        records.push(found.record);
    }
    Ok(records)
}

fn supersedable_record<'a>(
    index: &'a ProjectIndex,
    summary: &ScannedRecordSummary,
) -> Result<&'a Record, DadrError> {
    require_record_status(
        index,
        summary,
        RecordStatus::Accepted,
        "superseded",
        "supersede requires old record status `accepted`",
    )
}

fn accepted_successor_record<'a>(
    index: &'a ProjectIndex,
    summary: &ScannedRecordSummary,
) -> Result<&'a Record, DadrError> {
    require_record_status(
        index,
        summary,
        RecordStatus::Accepted,
        "supersede another record",
        "supersede requires new record status `accepted`",
    )
}

fn rewrite_status(content: &str, status: RecordStatus) -> Result<String, DadrError> {
    rewrite_front_matter(
        content,
        FrontMatterRewrite {
            title: None,
            tags: None,
            status: Some(status),
            append_successors: Vec::new(),
            remove_successors: false,
            body_markdown: None,
        },
    )
}

fn rewrite_for_converted_decision(content: &str) -> Result<String, DadrError> {
    rewrite_front_matter(
        content,
        FrontMatterRewrite {
            title: None,
            tags: None,
            status: Some(RecordStatus::Accepted),
            append_successors: Vec::new(),
            remove_successors: true,
            body_markdown: None,
        },
    )
}

fn rewrite_for_superseded_record(
    content: &str,
    successors: &[RecordId],
) -> Result<String, DadrError> {
    rewrite_front_matter(
        content,
        FrontMatterRewrite {
            title: None,
            tags: None,
            status: None,
            append_successors: successors.to_vec(),
            remove_successors: false,
            body_markdown: None,
        },
    )
}

fn rewrite_candidate_content(
    content: &str,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<String, DadrError> {
    rewrite_front_matter(
        content,
        FrontMatterRewrite {
            title: Some(title.to_owned()),
            tags: Some(tags.to_vec()),
            status: Some(RecordStatus::Candidate),
            append_successors: Vec::new(),
            remove_successors: false,
            body_markdown: Some(body_markdown.trim().to_owned()),
        },
    )
}

struct FrontMatterRewrite {
    title: Option<String>,
    tags: Option<Vec<String>>,
    status: Option<RecordStatus>,
    append_successors: Vec<RecordId>,
    remove_successors: bool,
    body_markdown: Option<String>,
}

fn rewrite_front_matter(content: &str, rewrite: FrontMatterRewrite) -> Result<String, DadrError> {
    let split = split_front_matter(content);
    let Some(raw_yaml) = split.raw_yaml else {
        return Err(DadrError::InvalidLifecycleTransition(
            "cannot rewrite front matter without YAML front matter".to_owned(),
        ));
    };
    if split.diagnostics.iter().any(Diagnostic::is_error) {
        return Err(DadrError::InvalidLifecycleTransition(
            "cannot rewrite malformed YAML front matter".to_owned(),
        ));
    }

    let mut mapping = match serde_yaml::from_str::<Value>(&raw_yaml)? {
        Value::Mapping(mapping) => mapping,
        _ => {
            return Err(DadrError::InvalidLifecycleTransition(
                "cannot rewrite front matter because it is not a mapping".to_owned(),
            ))
        }
    };

    if let Some(title) = rewrite.title {
        mapping.insert(Value::String("title".to_owned()), Value::String(title));
    }
    if let Some(status) = rewrite.status {
        mapping.insert(
            Value::String("status".to_owned()),
            Value::String(status.as_str().to_owned()),
        );
    }
    if let Some(tags) = rewrite.tags {
        let tag_key = Value::String("tags".to_owned());
        let normalized_tags = normalize_tags(&tags);
        if normalized_tags.is_empty() {
            mapping.remove(&tag_key);
        } else {
            mapping.insert(
                tag_key,
                Value::Sequence(normalized_tags.into_iter().map(Value::String).collect()),
            );
        }
    }
    if rewrite.remove_successors {
        mapping.remove(Value::String("successors".to_owned()));
    }
    for successor in rewrite.append_successors {
        append_successor(&mut mapping, successor)?;
    }

    let yaml = serde_yaml::to_string(&mapping)?
        .trim_start_matches("---\n")
        .to_owned();
    let body = match rewrite.body_markdown {
        Some(body_markdown) => {
            let mut body = String::from("\n");
            body.push_str(&body_markdown);
            body.push('\n');
            body
        }
        None => split.body,
    };
    Ok(format!("---\n{}---\n{}", yaml, body))
}

fn append_successor(mapping: &mut Mapping, successor: RecordId) -> Result<(), DadrError> {
    let key = Value::String("successors".to_owned());
    let successor_value = Value::String(successor.to_string());
    match mapping.get_mut(&key) {
        Some(Value::Sequence(values)) => {
            if !values.iter().any(|value| value == &successor_value) {
                values.push(successor_value);
            }
        }
        Some(_) => {
            return Err(DadrError::InvalidLifecycleTransition(
                "cannot append successor because `successors` is not an array".to_owned(),
            ))
        }
        None => {
            mapping.insert(key, Value::Sequence(vec![successor_value]));
        }
    }
    Ok(())
}

fn successor_graph(records: &[ScannedRecord]) -> BTreeMap<RecordId, Vec<RecordId>> {
    records
        .iter()
        .filter_map(|scanned| {
            let record = scanned.record.as_ref()?;
            Some((
                record.front_matter.id,
                record.front_matter.successors.clone(),
            ))
        })
        .collect()
}

fn reaches_record(
    records: &BTreeMap<RecordId, Vec<RecordId>>,
    current_id: RecordId,
    target_id: RecordId,
    visited: &mut BTreeSet<RecordId>,
) -> bool {
    if current_id == target_id {
        return true;
    }
    if !visited.insert(current_id) {
        return false;
    }

    let Some(successors) = records.get(&current_id) else {
        return false;
    };

    successors
        .iter()
        .copied()
        .any(|successor| reaches_record(records, successor, target_id, visited))
}

/// Allocates a UUIDv7 whose short id is not yet used in the project.
fn allocate_record_id(index: &ProjectIndex) -> RecordId {
    loop {
        let id = RecordId::new_v7();
        if !index.short_id_taken(&id.short_id()) {
            return id;
        }
        // The short id is the millisecond timestamp; wait for the next one.
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn record_filename(id: RecordId, title: &str) -> String {
    format!("{}-{}.md", id.short_id(), ascii_slug(title, 80, "untitled"))
}

fn render_candidate(id: RecordId, title: &str, tags: &[String], body_template: &str) -> String {
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str("title: ");
    output.push_str(&yaml_scalar(title));
    output.push('\n');
    output.push_str("status: candidate\n");
    output.push_str("id: ");
    output.push_str(&id.to_string());
    output.push('\n');

    let normalized_tags = normalize_tags(tags);
    if !normalized_tags.is_empty() {
        output.push_str("tags:\n");
        for tag in normalized_tags {
            output.push_str("  - ");
            output.push_str(&yaml_scalar(&tag));
            output.push('\n');
        }
    }

    output.push_str("---\n\n");
    output.push_str(body_template);
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn yaml_scalar(value: &str) -> String {
    if can_use_plain_yaml_scalar(value) {
        return value.to_owned();
    }
    if plain_yaml_scalar_chars(value) {
        return single_quoted_yaml_scalar(value);
    }
    serde_yaml::to_string(value)
        .expect("serializing string scalar cannot fail")
        .trim_start_matches("---\n")
        .trim_end()
        .to_owned()
}

fn can_use_plain_yaml_scalar(value: &str) -> bool {
    plain_yaml_scalar_chars(value) && !looks_like_yaml_resolved_scalar(value)
}

fn plain_yaml_scalar_chars(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_' | '/' | '.'))
}

fn looks_like_yaml_resolved_scalar(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "true"
            | "false"
            | "yes"
            | "no"
            | "on"
            | "off"
            | "null"
            | "~"
            | "candidate"
            | "accepted"
            | "rejected"
    ) || value.parse::<i64>().is_ok()
        || value.parse::<f64>().is_ok()
        || looks_like_iso_date(value)
}

fn looks_like_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..].iter().all(u8::is_ascii_digit)
}

fn single_quoted_yaml_scalar(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|tag| tag.trim().trim_start_matches('#').to_owned())
        .filter(|tag| !tag.is_empty())
        .collect()
}
