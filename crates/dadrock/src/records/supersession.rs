//! Successors named in `## Supersedes`: accepting them alone, as a merge, or
//! as a split, and finding the ones whose predecessor has moved on.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::{
    apply_record_writes, check_required_sections, relocate, require_record_status,
    resolve_unique_summaries, rewrite_for_converted_decision, rewrite_for_superseded_record,
    scanned_record_for_summary, supersedable_record, supersedes_section, with_supersedes_section,
    AcceptRecordResult, RecordMove, RecordWrite, Relocation, ScannedRecordSummary,
    SupersededRecordMove,
};
use crate::{
    error::DadrError,
    ids::RecordId,
    index::ProjectIndex,
    model::{Record, RecordStatus},
};

/// A predecessor named in `## Supersedes` that has been superseded since the
/// candidate was drafted, like a branch whose base has moved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleSupersedes {
    pub predecessor: ScannedRecordSummary,
    /// The decisions now in force in its place: the records reached through
    /// `successors` that have no successors of their own.
    pub current_successors: Vec<ScannedRecordSummary>,
}

/// What a candidate's `## Supersedes` section names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingSupersedes {
    /// Every named predecessor, in the order of the section.
    pub predecessors: Vec<ScannedRecordSummary>,
    /// The named predecessors that are no longer current.
    pub stale: Vec<StaleSupersedes>,
}

/// The records a candidate's `## Supersedes` section names, if any.
///
/// The section is ordinary Markdown, so the intent survives restarts and other
/// clients. Only a standalone `## Supersedes` section is a lifecycle request;
/// other references to records have no effect. Records other than candidates
/// have nothing pending.
///
/// A malformed section, or one that names a missing record, a record that is
/// not accepted, or the candidate itself, is an error. A predecessor that has
/// been superseded since is reported in [`PendingSupersedes::stale`] with the
/// decisions now in force in its place; accepting the candidate is refused
/// until the link is retargeted.
pub fn pending_supersedes(
    index: &ProjectIndex,
    query: &str,
) -> Result<PendingSupersedes, DadrError> {
    let found = index.resolve_one(query)?;
    if found.record.status != RecordStatus::Candidate {
        return Ok(PendingSupersedes::default());
    }
    let record = require_record_status(
        index,
        &found.record,
        RecordStatus::Candidate,
        "accepted",
        "a `## Supersedes` section requires a candidate",
    )?;

    let mut pending = PendingSupersedes::default();
    for predecessor in named_predecessors(index, &found.record, record)? {
        supersedable_record(index, &predecessor)?;
        if predecessor.superseded {
            pending.stale.push(StaleSupersedes {
                current_successors: current_successors(index, predecessor.id),
                predecessor: predecessor.clone(),
            });
        }
        pending.predecessors.push(predecessor);
    }
    Ok(pending)
}

/// The decisions in force in place of a record: the records reached through
/// `successors`, transitively, that have no successors of their own. A record
/// without successors is its own current version and yields nothing.
pub fn current_successors(index: &ProjectIndex, id: RecordId) -> Vec<ScannedRecordSummary> {
    let mut tips = Vec::new();
    let mut visited = BTreeSet::from([id]);
    let mut queue = VecDeque::from(successor_ids(index, id));
    while let Some(next) = queue.pop_front() {
        if !visited.insert(next) {
            continue;
        }
        let successors = successor_ids(index, next);
        if successors.is_empty() {
            if let Some(summary) = index.summary_by_id(next) {
                tips.push(summary.clone());
            }
        } else {
            queue.extend(successors);
        }
    }
    tips
}

fn successor_ids(index: &ProjectIndex, id: RecordId) -> Vec<RecordId> {
    index
        .scanned_by_id(id)
        .and_then(|scanned| scanned.record.as_ref())
        .map(|record| record.front_matter.successors.clone())
        .unwrap_or_default()
}

/// Accept one candidate. See [`accept_candidates`].
pub fn accept_candidate(
    index: &ProjectIndex,
    query: &str,
) -> Result<AcceptRecordResult, DadrError> {
    accept_candidates(index, &[query.to_owned()])
}

/// Accept a set of candidates in one operation.
///
/// Each candidate moves from the inbox to the ledger. Each predecessor named
/// in the `## Supersedes` section of a candidate in the set gains every
/// candidate in the set that names it, and moves to the archive: one candidate
/// naming several predecessors is a merge, several candidates naming the same
/// predecessor are a split. Candidates without `## Supersedes` are simply
/// accepted.
///
/// Every check happens before any write: each candidate meets the acceptance
/// requirements, and each named predecessor exists, is accepted, is not
/// superseded, and is not in the set. If a write fails, every record is
/// restored.
pub fn accept_candidates(
    index: &ProjectIndex,
    queries: &[String],
) -> Result<AcceptRecordResult, DadrError> {
    if queries.is_empty() {
        return Err(DadrError::InvalidInput(
            "accept requires at least one candidate query",
        ));
    }
    let candidates = resolve_unique_summaries(index, queries, "accept")?;
    let in_set: BTreeSet<RecordId> = candidates.iter().map(|candidate| candidate.id).collect();

    // Predecessors in the order they are first named, each with the
    // candidates of the set that name it.
    let mut order = Vec::new();
    let mut successors_of = BTreeMap::<RecordId, (ScannedRecordSummary, Vec<RecordId>)>::new();
    for candidate in &candidates {
        let record = acceptable_candidate(index, candidate)?;
        for predecessor in named_predecessors(index, candidate, record)? {
            if in_set.contains(&predecessor.id) {
                return Err(DadrError::InvalidLifecycleTransition(format!(
                    "`{}` names `{}` in `## Supersedes`, but `{}` is accepted in the same set; a candidate cannot supersede another candidate",
                    candidate.title, predecessor.title, predecessor.title
                )));
            }
            supersedable_record(index, &predecessor)?;
            if predecessor.superseded {
                return Err(already_superseded(index, &predecessor, candidate));
            }
            let entry = successors_of.entry(predecessor.id).or_insert_with(|| {
                order.push(predecessor.id);
                (predecessor.clone(), Vec::new())
            });
            entry.1.push(candidate.id);
        }
    }

    // Candidates and predecessors change in one operation, so either every
    // record changes or none does. Each keeps its filename; a successor can
    // take the name its predecessor leaves in the ledger.
    let project = index.project();
    let relocations: Vec<_> = candidates
        .iter()
        .map(|candidate| Relocation {
            from: &candidate.path,
            id: candidate.id,
            dir: project.ledger_dir(),
        })
        .chain(order.iter().map(|id| {
            let predecessor = &successors_of[id].0;
            Relocation {
                from: &predecessor.path,
                id: predecessor.id,
                dir: project.archive_dir(),
            }
        }))
        .collect();
    let mut targets = relocate(&relocations)?.into_iter();
    let mut writes = Vec::new();
    for (candidate, accepted_path) in candidates.iter().zip(targets.by_ref()) {
        writes.push(RecordWrite::read(
            &candidate.path,
            accepted_path,
            rewrite_for_converted_decision,
        )?);
    }
    for (id, archived_path) in order.iter().zip(targets) {
        let (predecessor, successors) = &successors_of[id];
        writes.push(RecordWrite::read(
            &predecessor.path,
            archived_path,
            |content| rewrite_for_superseded_record(content, successors),
        )?);
    }
    apply_record_writes(&writes)?;

    let (accepted_writes, superseded_writes) = writes.split_at(candidates.len());
    let accepted = candidates
        .iter()
        .zip(accepted_writes)
        .map(|(candidate, write)| RecordMove {
            id: candidate.id,
            from: write.from.clone(),
            to: write.to.clone(),
        })
        .collect();
    let superseded = order
        .iter()
        .zip(superseded_writes)
        .map(|(id, write)| SupersededRecordMove {
            id: *id,
            from: write.from.clone(),
            to: write.to.clone(),
            successors: successors_of[id].1.clone(),
        })
        .collect();
    Ok(AcceptRecordResult {
        accepted,
        superseded,
    })
}

/// The body of a new successor candidate: `body` with a `## Supersedes`
/// section first, linking each predecessor, labelled with its title, followed
/// by the reason, if any.
///
/// Each predecessor must be a current decision: accepted and not superseded.
/// A body that already has a `## Supersedes` section is kept as written when
/// the section names exactly the requested predecessors and any requested
/// reason matches its reason.
pub fn successor_candidate_body(
    index: &ProjectIndex,
    predecessor_queries: &[String],
    reason: Option<&str>,
    body: &str,
) -> Result<String, DadrError> {
    if predecessor_queries.is_empty() {
        return Err(DadrError::InvalidInput(
            "a successor must name at least one record it supersedes",
        ));
    }
    let predecessors = resolve_unique_summaries(index, predecessor_queries, "supersedes")?;
    for predecessor in &predecessors {
        if predecessor.status != RecordStatus::Accepted {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "`{}` is {}; a successor can supersede only an accepted decision",
                predecessor.title, predecessor.status
            )));
        }
        supersedable_record(index, predecessor)?;
        if predecessor.superseded {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "`{}` is already superseded by {}; draft a successor of the current decision instead",
                predecessor.title,
                record_list(&current_successors(index, predecessor.id))
            )));
        }
    }
    let reason = reason.map(str::trim).filter(|reason| !reason.is_empty());
    let wanted: BTreeSet<RecordId> = predecessors.iter().map(|record| record.id).collect();

    if let Some(section) = supersedes_section(body)? {
        if section.ids.iter().copied().collect::<BTreeSet<_>>() != wanted {
            return Err(DadrError::InvalidInput(
                "the `## Supersedes` section names different records than the ones to supersede",
            ));
        }
        if reason.is_some_and(|reason| section.reason.as_deref().map(str::trim) != Some(reason)) {
            return Err(DadrError::InvalidInput(
                "the body already has a `## Supersedes` section; write the reason there",
            ));
        }
        return Ok(body.to_owned());
    }

    let links: Vec<_> = predecessors
        .iter()
        .map(|record| (record.id, record.title.as_str()))
        .collect();
    let written = with_supersedes_section(body, &links, reason);
    // The reason is ordinary Markdown, but it cannot end the section early or
    // read as another link.
    let section = supersedes_section(&written)?;
    let round_trip = section.as_ref().is_some_and(|section| {
        section.ids.len() == links.len()
            && section
                .reason
                .as_deref()
                .map(|text| text.replace("\r\n", "\n"))
                == reason.map(|text| text.replace("\r\n", "\n"))
    });
    if !round_trip {
        return Err(DadrError::InvalidInput(
            "the reason for superseding cannot start with a record link or contain level-1 or level-2 headings",
        ));
    }
    Ok(written)
}

/// The records a `## Supersedes` section names, without checking their
/// status. A missing record or the candidate itself is an error.
fn named_predecessors(
    index: &ProjectIndex,
    candidate: &ScannedRecordSummary,
    record: &Record,
) -> Result<Vec<ScannedRecordSummary>, DadrError> {
    let Some(section) = supersedes_section(&record.body.raw_markdown)? else {
        return Ok(Vec::new());
    };
    section
        .ids
        .into_iter()
        .map(|id| {
            if id == candidate.id {
                return Err(DadrError::InvalidLifecycleTransition(
                    "a record cannot supersede itself",
                ));
            }
            index.summary_by_id(id).cloned().ok_or_else(|| {
                DadrError::InvalidLifecycleTransition(format!(
                    "`## Supersedes` in `{}` names `{id}`, which is not a record in this project",
                    candidate.title
                ))
            })
        })
        .collect()
}

/// A candidate that can be accepted: valid, with status `candidate`, and with
/// the four standard sections.
fn acceptable_candidate<'a>(
    index: &'a ProjectIndex,
    candidate: &ScannedRecordSummary,
) -> Result<&'a Record, DadrError> {
    let scanned = scanned_record_for_summary(index, candidate)?;
    if !scanned.is_valid() {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` has validation errors and cannot be accepted",
            candidate.path
        )));
    }
    let record = scanned.record.as_ref().ok_or_else(|| {
        DadrError::ProjectInvariant(format!("failed to parse `{}`", candidate.path))
    })?;
    if record.front_matter.status != RecordStatus::Candidate {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "accept requires status `candidate`, but `{}` is `{}`",
            candidate.title, record.front_matter.status
        )));
    }
    let sections = check_required_sections(&record.body.raw_markdown);
    if !sections.is_complete() {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` cannot be accepted: accept requires each standard section exactly once and with text: {}",
            candidate.title,
            sections.problems().join("; ")
        )));
    }
    Ok(record)
}

fn already_superseded(
    index: &ProjectIndex,
    predecessor: &ScannedRecordSummary,
    candidate: &ScannedRecordSummary,
) -> DadrError {
    DadrError::InvalidLifecycleTransition(format!(
        "`{}` is already superseded by {}; retarget `## Supersedes` in `{}` at the current decision, or reject the candidate",
        predecessor.title,
        record_list(&current_successors(index, predecessor.id)),
        candidate.title
    ))
}

/// Records named for a message: "`Title` (short id)", joined by commas.
pub(crate) fn record_list(records: &[ScannedRecordSummary]) -> String {
    if records.is_empty() {
        return "records that are no longer in the project".to_owned();
    }
    records
        .iter()
        .map(|record| format!("`{}` ({})", record.title, record.id.short_id()))
        .collect::<Vec<_>>()
        .join(", ")
}
