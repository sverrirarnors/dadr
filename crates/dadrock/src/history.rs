//! The history of one decision: its versions, the supersession links between
//! them, the records at the other end of a link that are not versions, and
//! each version's text edits.
//!
//! The versions and links are derived from `successors` alone: decision
//! history lives in the record set. A link's reason comes from the successor's
//! `## Supersedes` section. Text edits, the corrections made to a version
//! after it was accepted or rejected, are read from Git (see
//! [`crate::edits`]) and never stored in records. Diffs compare the title, the
//! tags, and the body without `## Supersedes`, after normalizing line endings;
//! they are computed when asked for and never stored.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub use crate::edits::RecordEdit;
use crate::{
    commands::{text_diff_lines, CommandDiagnostic, TextDiffKind, TextDiffLine},
    edits::EditReader,
    error::{DadrError, Severity},
    index::ProjectIndex,
    model::{Record, RecordStatus},
    records::{body_without_supersedes, supersedes_section},
    text::normalized_tag_key,
    RecordId,
};

/// Lines of unchanged text kept around each change in text output.
const DIFF_CONTEXT_LINES: usize = 3;

/// One record as it appears in a history or a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: RecordId,
    pub short_id: String,
    pub title: String,
    pub status: RecordStatus,
    /// An accepted record with non-empty `successors`.
    pub superseded: bool,
    /// No record supersedes this one: none of its `successors` names a record
    /// in the project.
    pub current: bool,
    pub path: Utf8PathBuf,
    pub tags: Vec<String>,
}

/// A version of the decision, with its body for display and comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryVersion {
    #[serde(flatten)]
    pub record: HistoryRecord,
    /// The body without `## Supersedes`, with `\n` line endings and without
    /// leading or trailing blank lines.
    pub body: String,
    /// Edits to the title or body since the version was accepted or rejected,
    /// read from Git, newest first; an uncommitted edit in the working tree
    /// comes first. Empty for candidates; `null` when Git cannot be read.
    pub edits: Option<Vec<RecordEdit>>,
}

/// One supersession edge: `successor` takes the place of `predecessor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryLink {
    pub predecessor: RecordId,
    pub successor: RecordId,
    /// The reason from the successor's `## Supersedes` section, when that
    /// section names the predecessor.
    pub reason: Option<String>,
    /// The change from the predecessor to the successor, when asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<RecordDiff>,
}

/// The history of the decision that contains `record`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordHistory {
    /// The record the history was opened at.
    pub record: RecordId,
    /// The opened record, its predecessors back to the first version, and its
    /// successors forward to the current ones: newest first, every record
    /// above its predecessors.
    pub versions: Vec<HistoryVersion>,
    /// Every supersession edge that touches a version, in display order.
    pub links: Vec<HistoryLink>,
    /// Records at the other end of a link that are not versions: the other
    /// half of a split, the other input of a merge. Newest first.
    pub neighbors: Vec<HistoryRecord>,
}

/// A title change between two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleChange {
    pub old: String,
    pub new: String,
}

/// The difference between two records: title, tags, and the body without
/// `## Supersedes`. Other front matter is not compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDiff {
    /// Whether the title, the tags, or the body differ.
    pub changed: bool,
    /// The title change, or `None` when the title is the same.
    pub title: Option<TitleChange>,
    pub tags_added: Vec<String>,
    pub tags_removed: Vec<String>,
    /// A line diff of the bodies, including unchanged lines.
    pub lines: Vec<TextDiffLine>,
}

/// Two records and the difference from `old` to `new`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDiffResult {
    pub old: HistoryRecord,
    pub new: HistoryRecord,
    pub diff: RecordDiff,
}

/// Build the history of the record matching `query`. With `patch`, every link
/// and every text edit carries its diff. Problems in the data (unresolved
/// successors, cycles, invalid `## Supersedes` sections) are returned as
/// warnings; the history is still built.
pub fn record_history(
    index: &ProjectIndex,
    query: &str,
    patch: bool,
) -> Result<(RecordHistory, Vec<CommandDiagnostic>), DadrError> {
    let opened = index.resolve_one(query)?.record.id;
    let graph = SupersessionGraph::new(index);
    let mut diagnostics = Vec::new();

    let mut versions = graph.reachable(opened, Direction::Predecessors);
    versions.extend(graph.reachable(opened, Direction::Successors));
    let (order, cycle) = graph.display_order(&versions);

    for id in &order {
        for missing in graph.unresolved.get(id).into_iter().flatten() {
            diagnostics.push(warning(
                "unresolved_successor",
                format!(
                    "`{id}` lists missing successor `{missing}`; it is left out of the history"
                ),
                index,
                *id,
            ));
        }
    }
    if cycle || order.iter().any(|id| graph.successors(*id).contains(id)) {
        diagnostics.push(CommandDiagnostic {
            severity: Severity::Warning,
            code: "successor_cycle".to_owned(),
            message: "the supersession links of this history form a cycle; the order shown breaks it arbitrarily".to_owned(),
            path: None,
            field: None,
        });
    }

    let mut link_set = BTreeSet::new();
    for &id in &order {
        for &successor in graph.successors(id) {
            if successor != id {
                link_set.insert((id, successor));
            }
        }
        for &predecessor in graph.predecessors(id) {
            if predecessor != id {
                link_set.insert((predecessor, id));
            }
        }
    }

    let mut neighbor_ids: Vec<RecordId> = link_set
        .iter()
        .flat_map(|(predecessor, successor)| [*predecessor, *successor])
        .filter(|id| !versions.contains(id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    neighbor_ids.reverse();

    let rank: BTreeMap<RecordId, usize> = order
        .iter()
        .chain(&neighbor_ids)
        .enumerate()
        .map(|(rank, id)| (*id, rank))
        .collect();
    let mut link_list: Vec<(RecordId, RecordId)> = link_set.into_iter().collect();
    link_list.sort_by_key(|(predecessor, successor)| (rank[successor], rank[predecessor]));

    let mut reasons = ReasonCache::default();
    let mut links = Vec::with_capacity(link_list.len());
    for (predecessor, successor) in link_list {
        let reason = reasons.reason(index, predecessor, successor, &mut diagnostics);
        let diff = if patch {
            match (graph.record(predecessor), graph.record(successor)) {
                (Some(old), Some(new)) => Some(record_diff(old, new)),
                _ => None,
            }
        } else {
            None
        };
        links.push(HistoryLink {
            predecessor,
            successor,
            reason,
            diff,
        });
    }

    let edits = EditReader::new(&index.project().project_dir);
    let versions = order
        .iter()
        .filter_map(|id| {
            let record = graph.record(*id)?;
            let summary = graph.history_record(*id)?;
            Some(HistoryVersion {
                edits: edits.edits(record, &summary.path, patch),
                body: comparable_body(&record.body.raw_markdown),
                record: summary,
            })
        })
        .collect();
    let neighbors = neighbor_ids
        .iter()
        .filter_map(|id| graph.history_record(*id))
        .collect();

    Ok((
        RecordHistory {
            record: opened,
            versions,
            links,
            neighbors,
        },
        diagnostics,
    ))
}

/// Compare any two records: `old` to `new`.
pub fn diff_records(
    index: &ProjectIndex,
    old_query: &str,
    new_query: &str,
) -> Result<RecordDiffResult, DadrError> {
    let old_id = index.resolve_one(old_query)?.record.id;
    let new_id = index.resolve_one(new_query)?.record.id;
    let graph = SupersessionGraph::new(index);
    let missing = |id: RecordId| {
        DadrError::ProjectInvariant(format!("record `{id}` resolved but has no parsed record"))
    };
    let old = graph.record(old_id).ok_or_else(|| missing(old_id))?;
    let new = graph.record(new_id).ok_or_else(|| missing(new_id))?;
    Ok(RecordDiffResult {
        old: graph
            .history_record(old_id)
            .ok_or_else(|| missing(old_id))?,
        new: graph
            .history_record(new_id)
            .ok_or_else(|| missing(new_id))?,
        diff: record_diff(old, new),
    })
}

/// The difference from `old` to `new`: title, tags, and the body without
/// `## Supersedes` after normalizing line endings.
pub fn record_diff(old: &Record, new: &Record) -> RecordDiff {
    let title = (old.front_matter.title != new.front_matter.title).then(|| TitleChange {
        old: old.front_matter.title.clone(),
        new: new.front_matter.title.clone(),
    });
    let old_tags = record_tags(old);
    let new_tags = record_tags(new);
    let tags_added = tags_missing_from(&new_tags, &old_tags);
    let tags_removed = tags_missing_from(&old_tags, &new_tags);
    let lines = text_diff_lines(
        &comparable_body(&old.body.raw_markdown),
        &comparable_body(&new.body.raw_markdown),
    );
    let changed = title.is_some()
        || !tags_added.is_empty()
        || !tags_removed.is_empty()
        || lines.iter().any(|line| line.kind != TextDiffKind::Context);
    RecordDiff {
        changed,
        title,
        tags_added,
        tags_removed,
        lines,
    }
}

/// The body as versions are compared: without `## Supersedes`, with `\n`
/// line endings, and without leading or trailing blank lines.
pub fn comparable_body(body: &str) -> String {
    let normalized = body.replace("\r\n", "\n").replace('\r', "\n");
    let without = body_without_supersedes(&normalized);
    let mut lines: Vec<&str> = without.lines().collect();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        String::new()
    } else {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }
}

fn record_tags(record: &Record) -> Vec<String> {
    record
        .front_matter
        .tags
        .iter()
        .map(|tag| tag.as_str().trim().to_owned())
        .filter(|tag| !tag.is_empty())
        .collect()
}

/// Tags in `tags` whose normalized form is not in `other`, without duplicates.
fn tags_missing_from(tags: &[String], other: &[String]) -> Vec<String> {
    let other: BTreeSet<String> = other.iter().map(|tag| normalized_tag_key(tag)).collect();
    let mut seen = BTreeSet::new();
    tags.iter()
        .filter(|tag| {
            let key = normalized_tag_key(tag);
            !other.contains(&key) && seen.insert(key)
        })
        .cloned()
        .collect()
}

fn warning(code: &str, message: String, index: &ProjectIndex, id: RecordId) -> CommandDiagnostic {
    CommandDiagnostic {
        severity: Severity::Warning,
        code: code.to_owned(),
        message,
        path: index.summary_by_id(id).map(|summary| summary.path.clone()),
        field: None,
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Predecessors,
    Successors,
}

/// The resolved supersession edges of a project. Successor ids that match no
/// record are kept aside, not followed.
struct SupersessionGraph<'a> {
    index: &'a ProjectIndex,
    successors: BTreeMap<RecordId, Vec<RecordId>>,
    predecessors: BTreeMap<RecordId, Vec<RecordId>>,
    unresolved: BTreeMap<RecordId, Vec<RecordId>>,
}

impl<'a> SupersessionGraph<'a> {
    fn new(index: &'a ProjectIndex) -> Self {
        let mut graph = Self {
            index,
            successors: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            unresolved: BTreeMap::new(),
        };
        let ids: BTreeSet<RecordId> = index.summaries().iter().map(|summary| summary.id).collect();
        for id in ids {
            let Some(record) = graph.record(id) else {
                continue;
            };
            for &successor in &record.front_matter.successors {
                if index.summary_by_id(successor).is_none() {
                    push_unique(graph.unresolved.entry(id).or_default(), successor);
                    continue;
                }
                push_unique(graph.successors.entry(id).or_default(), successor);
                push_unique(graph.predecessors.entry(successor).or_default(), id);
            }
        }
        graph
    }

    fn record(&self, id: RecordId) -> Option<&'a Record> {
        self.index.scanned_by_id(id)?.record.as_ref()
    }

    fn successors(&self, id: RecordId) -> &[RecordId] {
        self.successors.get(&id).map_or(&[], Vec::as_slice)
    }

    fn predecessors(&self, id: RecordId) -> &[RecordId] {
        self.predecessors.get(&id).map_or(&[], Vec::as_slice)
    }

    fn history_record(&self, id: RecordId) -> Option<HistoryRecord> {
        let summary = self.index.summary_by_id(id)?;
        let record = self.record(id)?;
        Some(HistoryRecord {
            id,
            short_id: id.short_id(),
            title: summary.title.clone(),
            status: summary.status,
            superseded: summary.superseded,
            current: self.successors(id).is_empty(),
            path: summary.path.clone(),
            tags: record_tags(record),
        })
    }

    /// `start` and every record reachable from it in one direction. Visited
    /// records are never revisited, so cycles terminate.
    fn reachable(&self, start: RecordId, direction: Direction) -> BTreeSet<RecordId> {
        let mut seen = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(id) = queue.pop_front() {
            let next = match direction {
                Direction::Predecessors => self.predecessors(id),
                Direction::Successors => self.successors(id),
            };
            for &other in next {
                if seen.insert(other) {
                    queue.push_back(other);
                }
            }
        }
        seen
    }

    /// Versions newest first, every record above its predecessors; among the
    /// records free to come next, the one with the latest id comes first. A
    /// cycle is broken at its newest record; the flag reports that it was.
    fn display_order(&self, versions: &BTreeSet<RecordId>) -> (Vec<RecordId>, bool) {
        let mut waiting: BTreeMap<RecordId, usize> = versions
            .iter()
            .map(|id| {
                let count = self
                    .successors(*id)
                    .iter()
                    .filter(|successor| *successor != id && versions.contains(*successor))
                    .count();
                (*id, count)
            })
            .collect();
        let mut ready: BTreeSet<RecordId> = waiting
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut order = Vec::with_capacity(versions.len());
        let mut cycle = false;
        while !waiting.is_empty() {
            let next = match ready.pop_last() {
                Some(id) => id,
                None => {
                    cycle = true;
                    match waiting.keys().next_back() {
                        Some(id) => *id,
                        None => break,
                    }
                }
            };
            waiting.remove(&next);
            order.push(next);
            for predecessor in self.predecessors(next) {
                if let Some(count) = waiting.get_mut(predecessor) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.insert(*predecessor);
                    }
                }
            }
        }
        (order, cycle)
    }
}

fn push_unique(list: &mut Vec<RecordId>, id: RecordId) {
    if !list.contains(&id) {
        list.push(id);
    }
}

/// Parsed `## Supersedes` sections, read once per successor.
#[derive(Default)]
struct ReasonCache {
    sections: BTreeMap<RecordId, Option<crate::records::Supersedes>>,
}

impl ReasonCache {
    fn reason(
        &mut self,
        index: &ProjectIndex,
        predecessor: RecordId,
        successor: RecordId,
        diagnostics: &mut Vec<CommandDiagnostic>,
    ) -> Option<String> {
        let section = self.sections.entry(successor).or_insert_with(|| {
            let record = index.scanned_by_id(successor)?.record.as_ref()?;
            match supersedes_section(&record.body.raw_markdown) {
                Ok(section) => section,
                Err(error) => {
                    diagnostics.push(warning(
                        "invalid_supersedes",
                        format!("the reason for `{successor}` cannot be read: {error}"),
                        index,
                        successor,
                    ));
                    None
                }
            }
        });
        section
            .as_ref()
            .filter(|section| section.ids.contains(&predecessor))
            .and_then(|section| section.reason.clone())
    }
}

/// Render a history as a lane graph, newest first, like `git log --graph`.
///
/// `●` is a version and `◉` the version the history was opened at; `│`
/// carries a link down to a predecessor, `├─╯` joins the successors of a split
/// at their predecessor, and `├─╮` opens the predecessors of a merge. A link's
/// reason is printed where the link is drawn, followed by its diff when the
/// history was built with `patch`. A version with text edits gets a line such
/// as `edited 2 times since accepted (latest 2026-10-01, Alice)` under it,
/// followed by each edit's diff with `patch`. Neighbors are listed under the
/// version they link to.
pub fn render_history_text(history: &RecordHistory) -> String {
    HistoryGraphText::new(history).render()
}

/// Render a diff of two records: a header, the title and tag changes, and the
/// changed body lines with some context.
pub fn render_diff_text(result: &RecordDiffResult) -> String {
    let mut lines = vec![
        format!("--- {}  {}", result.old.short_id, result.old.title),
        format!("+++ {}  {}", result.new.short_id, result.new.title),
    ];
    lines.extend(diff_text_lines(&result.diff));
    finish_lines(lines)
}

fn finish_lines(lines: Vec<String>) -> String {
    let mut text = String::new();
    for line in lines {
        text.push_str(line.trim_end());
        text.push('\n');
    }
    text
}

/// The title line, the tag summary, and the changed body lines of a diff,
/// with a few unchanged lines around each change.
pub fn diff_text_lines(diff: &RecordDiff) -> Vec<String> {
    if !diff.changed {
        return vec!["(no changes)".to_owned()];
    }
    let mut lines = Vec::new();
    if let Some(title) = &diff.title {
        lines.push(format!("title: {} → {}", title.old, title.new));
    }
    if !diff.tags_added.is_empty() || !diff.tags_removed.is_empty() {
        let tags: Vec<String> = diff
            .tags_added
            .iter()
            .map(|tag| format!("+{tag}"))
            .chain(diff.tags_removed.iter().map(|tag| format!("−{tag}")))
            .collect();
        lines.push(format!("tags: {}", tags.join(" ")));
    }
    for line in diff_hunks(&diff.lines, DIFF_CONTEXT_LINES) {
        lines.push(match line {
            None => "⋯".to_owned(),
            Some(line) => {
                let marker = match line.kind {
                    TextDiffKind::Context => ' ',
                    TextDiffKind::Added => '+',
                    TextDiffKind::Removed => '-',
                };
                format!("{marker} {}", line.text)
            }
        });
    }
    lines
}

/// Changed lines with up to `context` unchanged lines around them; `None`
/// separates lines that are not adjacent.
fn diff_hunks(lines: &[TextDiffLine], context: usize) -> Vec<Option<&TextDiffLine>> {
    let mut keep = vec![false; lines.len()];
    for (index, line) in lines.iter().enumerate() {
        if line.kind != TextDiffKind::Context {
            let start = index.saturating_sub(context);
            let end = (index + context).min(lines.len().saturating_sub(1));
            keep[start..=end].fill(true);
        }
    }
    let mut hunks = Vec::new();
    let mut previous = None;
    for (index, line) in lines.iter().enumerate() {
        if !keep[index] {
            continue;
        }
        if previous.is_some_and(|previous| index > previous + 1) {
            hunks.push(None);
        }
        hunks.push(Some(line));
        previous = Some(index);
    }
    hunks
}

/// One cell of the lane graph: the lane's glyph and the character after it.
type Cell = [char; 2];

enum RowText {
    Version(usize),
    Line(String),
    Empty,
}

struct Row {
    cells: Vec<Cell>,
    text: RowText,
}

struct HistoryGraphText<'a> {
    history: &'a RecordHistory,
    position: BTreeMap<RecordId, usize>,
    /// Version predecessors and successors of each version, in display order.
    predecessors: BTreeMap<RecordId, Vec<RecordId>>,
    successors: BTreeMap<RecordId, Vec<RecordId>>,
    links: BTreeMap<(RecordId, RecordId), &'a HistoryLink>,
    /// One-line entries for the neighbors linked to each version.
    neighbor_lines: BTreeMap<RecordId, Vec<String>>,
    short_ids: BTreeMap<RecordId, &'a str>,
}

impl<'a> HistoryGraphText<'a> {
    fn new(history: &'a RecordHistory) -> Self {
        let position: BTreeMap<RecordId, usize> = history
            .versions
            .iter()
            .enumerate()
            .map(|(index, version)| (version.record.id, index))
            .collect();
        let neighbors: BTreeMap<RecordId, &HistoryRecord> = history
            .neighbors
            .iter()
            .map(|neighbor| (neighbor.id, neighbor))
            .collect();
        let short_ids = history
            .versions
            .iter()
            .map(|version| &version.record)
            .chain(&history.neighbors)
            .map(|record| (record.id, record.short_id.as_str()))
            .collect();

        let mut text = Self {
            history,
            position,
            predecessors: BTreeMap::new(),
            successors: BTreeMap::new(),
            links: BTreeMap::new(),
            neighbor_lines: BTreeMap::new(),
            short_ids,
        };
        for link in &history.links {
            text.links.insert((link.predecessor, link.successor), link);
            let known_predecessor = text.position.contains_key(&link.predecessor);
            let known_successor = text.position.contains_key(&link.successor);
            match (known_predecessor, known_successor) {
                (true, true) => {
                    text.predecessors
                        .entry(link.successor)
                        .or_default()
                        .push(link.predecessor);
                    text.successors
                        .entry(link.predecessor)
                        .or_default()
                        .push(link.successor);
                }
                (true, false) => {
                    if let Some(neighbor) = neighbors.get(&link.successor) {
                        text.neighbor_lines
                            .entry(link.predecessor)
                            .or_default()
                            .push(format!(
                                "also became {}  {}",
                                neighbor.short_id, neighbor.title
                            ));
                    }
                }
                (false, true) => {
                    if let Some(neighbor) = neighbors.get(&link.predecessor) {
                        text.neighbor_lines
                            .entry(link.successor)
                            .or_default()
                            .push(format!(
                                "merged with {}  {}",
                                neighbor.short_id, neighbor.title
                            ));
                    }
                }
                (false, false) => {}
            }
        }
        let position = text.position.clone();
        for list in text
            .predecessors
            .values_mut()
            .chain(text.successors.values_mut())
        {
            list.sort_by_key(|id| position[id]);
            list.dedup();
        }
        text
    }

    fn short_id(&self, id: RecordId) -> String {
        self.short_ids
            .get(&id)
            .map_or_else(|| id.short_id(), |short| (*short).to_owned())
    }

    /// Version predecessors of `id` drawn below it (all of them, unless a
    /// cycle put one above).
    fn lower_predecessors(&self, id: RecordId) -> Vec<RecordId> {
        let position = self.position[&id];
        self.predecessors
            .get(&id)
            .into_iter()
            .flatten()
            .copied()
            .filter(|predecessor| self.position[predecessor] > position)
            .collect()
    }

    /// The links whose notes are printed just above `id`: from `id` to each
    /// successor above it whose only version predecessor is `id`.
    fn links_above(&self, id: RecordId) -> Vec<&'a HistoryLink> {
        let position = self.position[&id];
        self.successors
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|successor| self.position[*successor] < position)
            .filter(|successor| self.lower_predecessors(**successor).len() == 1)
            .filter_map(|successor| self.links.get(&(id, *successor)).copied())
            .collect()
    }

    /// The reasons of `links`, once each, then their diffs when present.
    fn note_lines(&self, links: &[&HistoryLink]) -> Vec<String> {
        let mut reasons: Vec<(RecordId, &str)> = Vec::new();
        for link in links {
            if let Some(reason) = link.reason.as_deref() {
                if !reasons.iter().any(|(_, seen)| *seen == reason) {
                    reasons.push((link.successor, reason));
                }
            }
        }
        let attribute = reasons.len() > 1;
        let mut lines = Vec::new();
        for (successor, reason) in reasons {
            for (index, line) in reason.lines().enumerate() {
                if attribute && index == 0 {
                    lines.push(format!("{}: {line}", self.short_id(successor)));
                } else {
                    lines.push(line.to_owned());
                }
            }
        }
        for link in links {
            if let Some(diff) = &link.diff {
                lines.push(format!(
                    "diff {} → {}",
                    self.short_id(link.predecessor),
                    self.short_id(link.successor)
                ));
                lines.extend(diff_text_lines(diff));
            }
        }
        lines
    }

    fn render(&self) -> String {
        let rows = self.layout();
        let lanes = rows.iter().map(|row| row.cells.len()).max().unwrap_or(0);
        let graph_width = lanes.max(2) * 2;
        let title_width = self
            .history
            .versions
            .iter()
            .map(|version| version.record.title.chars().count())
            .max()
            .unwrap_or(0);

        let mut lines = Vec::with_capacity(rows.len());
        for row in rows {
            let mut line: String = row.cells.iter().flatten().collect();
            let used = row.cells.len() * 2;
            line.extend(std::iter::repeat_n(' ', graph_width.saturating_sub(used)));
            match row.text {
                RowText::Version(index) => {
                    line.push_str(&self.version_text(index, title_width));
                }
                RowText::Line(text) => line.push_str(&text),
                RowText::Empty => {}
            }
            lines.push(line);
        }
        finish_lines(lines)
    }

    fn version_text(&self, index: usize, title_width: usize) -> String {
        let record = &self.history.versions[index].record;
        let mut labels = Vec::new();
        if record.status != RecordStatus::Accepted {
            labels.push(record.status.as_str());
        }
        if record.current {
            labels.push("current");
        }
        if labels.is_empty() {
            format!("{}  {}", record.short_id, record.title)
        } else {
            let padding = title_width.saturating_sub(record.title.chars().count());
            format!(
                "{}  {}{}  {}",
                record.short_id,
                record.title,
                " ".repeat(padding),
                labels.join("  ")
            )
        }
    }

    fn layout(&self) -> Vec<Row> {
        let mut lanes: Vec<Option<RecordId>> = Vec::new();
        let mut rows = Vec::new();
        for (index, version) in self.history.versions.iter().enumerate() {
            let id = version.record.id;
            let awaiting: Vec<usize> = lanes
                .iter()
                .enumerate()
                .filter(|(_, lane)| **lane == Some(id))
                .map(|(lane, _)| lane)
                .collect();
            let mut above = self.note_lines(&self.links_above(id)).into_iter();

            let column = match awaiting.split_first() {
                None => free_lane(&mut lanes, 0),
                Some((&first, others)) => {
                    if !others.is_empty() {
                        rows.push(Row {
                            cells: join_cells(&lanes, first, others),
                            text: line_text(above.next()),
                        });
                        for &other in others {
                            lanes[other] = None;
                        }
                    }
                    first
                }
            };
            for line in above {
                rows.push(connector(&lanes, line_text(Some(line))));
            }

            lanes[column] = Some(id);
            let mut cells = lane_cells(&lanes);
            cells[column] = [
                if id == self.history.record {
                    '◉'
                } else {
                    '●'
                },
                ' ',
            ];
            rows.push(Row {
                cells,
                text: RowText::Version(index),
            });

            let predecessors = self.lower_predecessors(id);
            lanes[column] = predecessors.first().copied();
            for line in edit_lines(version) {
                rows.push(connector(&lanes, line_text(Some(line))));
            }
            if let Some(rest) = predecessors.get(1..).filter(|rest| !rest.is_empty()) {
                let opened: Vec<usize> = rest
                    .iter()
                    .map(|predecessor| {
                        let lane = free_lane(&mut lanes, column + 1);
                        lanes[lane] = Some(*predecessor);
                        lane
                    })
                    .collect();
                let links: Vec<&HistoryLink> = predecessors
                    .iter()
                    .filter_map(|predecessor| self.links.get(&(*predecessor, id)).copied())
                    .collect();
                let mut notes = self.note_lines(&links).into_iter();
                rows.push(Row {
                    cells: fork_cells(&lanes, column, &opened),
                    text: line_text(notes.next()),
                });
                for line in notes {
                    rows.push(connector(&lanes, line_text(Some(line))));
                }
            }
            for line in self.neighbor_lines.get(&id).into_iter().flatten() {
                rows.push(connector(&lanes, RowText::Line(line.clone())));
            }
            while lanes.last().is_some_and(Option::is_none) {
                lanes.pop();
            }
        }
        rows
    }
}

/// The summary of a version's text edits and, when present, each edit's
/// diff under a header naming its commit.
fn edit_lines(version: &HistoryVersion) -> Vec<String> {
    let Some(edits) = version.edits.as_deref() else {
        return Vec::new();
    };
    let Some(latest) = edits.first() else {
        return Vec::new();
    };
    let since = if version.record.status == RecordStatus::Rejected {
        "rejected"
    } else {
        "accepted"
    };
    let times = match edits.len() {
        1 => "1 time".to_owned(),
        count => format!("{count} times"),
    };
    let latest = match (&latest.date, &latest.author) {
        (Some(date), Some(author)) => {
            format!("latest {}, {author}", date.get(..10).unwrap_or(date))
        }
        _ => "latest not committed yet".to_owned(),
    };
    let mut lines = vec![format!("edited {times} since {since} ({latest})")];
    for edit in edits {
        let Some(diff) = &edit.diff else {
            continue;
        };
        lines.push(match (&edit.short_commit, &edit.subject) {
            (Some(commit), Some(subject)) => format!("edit {commit}  {subject}"),
            (Some(commit), None) => format!("edit {commit}"),
            _ => "edit (not committed yet)".to_owned(),
        });
        lines.extend(diff_text_lines(diff));
    }
    lines
}

fn line_text(line: Option<String>) -> RowText {
    match line {
        Some(line) if !line.trim().is_empty() => RowText::Line(line),
        _ => RowText::Empty,
    }
}

/// The first free lane at or after `start`, adding one when none is free.
fn free_lane(lanes: &mut Vec<Option<RecordId>>, start: usize) -> usize {
    if let Some(lane) = (start..lanes.len()).find(|lane| lanes[*lane].is_none()) {
        return lane;
    }
    while lanes.len() < start {
        lanes.push(None);
    }
    lanes.push(None);
    lanes.len() - 1
}

fn lane_cells(lanes: &[Option<RecordId>]) -> Vec<Cell> {
    lanes
        .iter()
        .map(|lane| [if lane.is_some() { '│' } else { ' ' }, ' '])
        .collect()
}

fn connector(lanes: &[Option<RecordId>], text: RowText) -> Row {
    Row {
        cells: lane_cells(lanes),
        text,
    }
}

/// `├─╯`: the lanes in `others` end by joining the lane `target` on their left.
fn join_cells(lanes: &[Option<RecordId>], target: usize, others: &[usize]) -> Vec<Cell> {
    span_cells(lanes, target, others, '┴', '╯')
}

/// `├─╮`: the lanes in `opened` start from the lane `source` on their left.
fn fork_cells(lanes: &[Option<RecordId>], source: usize, opened: &[usize]) -> Vec<Cell> {
    span_cells(lanes, source, opened, '┬', '╮')
}

fn span_cells(
    lanes: &[Option<RecordId>],
    anchor: usize,
    ends: &[usize],
    middle: char,
    last: char,
) -> Vec<Cell> {
    let mut cells = lane_cells(lanes);
    let Some(&end) = ends.iter().max() else {
        return cells;
    };
    cells[anchor] = ['├', '─'];
    for (lane, cell) in cells.iter_mut().enumerate().take(end).skip(anchor + 1) {
        let glyph = if ends.contains(&lane) {
            middle
        } else if lanes[lane].is_some() {
            '┼'
        } else {
            '─'
        };
        *cell = [glyph, '─'];
    }
    cells[end] = [last, ' '];
    cells
}
