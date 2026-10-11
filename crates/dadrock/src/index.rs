use std::collections::BTreeMap;

use crate::{
    error::DadrError,
    ids::{RecordId, RecordIdPrefix, RecordRef},
    model::RecordStatus,
    project::ProjectContext,
    records::{
        scan_records, FindRecordsResult, FindResolution, FoundRecord, MatchKind, ScannedRecord,
        ScannedRecordSummary,
    },
};

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    project: ProjectContext,
    scanned: Vec<ScannedRecord>,
    summaries: Vec<ScannedRecordSummary>,
    scanned_by_id: BTreeMap<RecordId, usize>,
    summary_by_id: BTreeMap<RecordId, ScannedRecordSummary>,
}

impl ProjectIndex {
    pub fn scan(project: &ProjectContext) -> Result<Self, DadrError> {
        let mut scanned = Vec::new();
        for dir in project.record_dirs() {
            scanned.extend(scan_records(&dir)?);
        }
        Ok(Self::from_scanned(project.clone(), scanned))
    }

    pub fn from_scanned(project: ProjectContext, scanned: Vec<ScannedRecord>) -> Self {
        let mut summaries: Vec<_> = scanned.iter().filter_map(ScannedRecord::summary).collect();
        summaries.sort_by(|left, right| {
            left.timestamp_ms
                .cmp(&right.timestamp_ms)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.path.cmp(&right.path))
        });
        let mut scanned_by_id = BTreeMap::new();
        let mut summary_by_id = BTreeMap::new();

        for (index, scanned_record) in scanned.iter().enumerate() {
            let Some(summary) = scanned_record.summary() else {
                continue;
            };
            scanned_by_id.entry(summary.id).or_insert(index);
            summary_by_id.entry(summary.id).or_insert(summary);
        }

        Self {
            project,
            scanned,
            summaries,
            scanned_by_id,
            summary_by_id,
        }
    }

    pub fn project(&self) -> &ProjectContext {
        &self.project
    }

    pub fn scanned_records(&self) -> &[ScannedRecord] {
        &self.scanned
    }

    pub fn summaries(&self) -> &[ScannedRecordSummary] {
        &self.summaries
    }

    pub fn scanned_by_id(&self, id: RecordId) -> Option<&ScannedRecord> {
        self.scanned_by_id
            .get(&id)
            .and_then(|index| self.scanned.get(*index))
    }

    pub fn summary_by_id(&self, id: RecordId) -> Option<&ScannedRecordSummary> {
        self.summary_by_id.get(&id)
    }

    pub fn list_records(&self, status: Option<RecordStatus>) -> Vec<ScannedRecordSummary> {
        self.summaries
            .iter()
            .filter(|summary| {
                status
                    .as_ref()
                    .is_none_or(|wanted| &summary.status == wanted)
            })
            .cloned()
            .collect()
    }

    pub fn find_records(&self, query: &str) -> FindRecordsResult {
        let query = query.trim();
        if query.is_empty() {
            return find_result(query, Vec::new());
        }

        if let Ok(id) = RecordId::parse(query) {
            let matches = self
                .summaries
                .iter()
                .filter(|summary| summary.id == id)
                .cloned()
                .map(|record| FoundRecord {
                    match_kind: MatchKind::ExactId,
                    record,
                })
                .collect();
            return find_result(query, matches);
        }

        if let Ok(prefix) = RecordIdPrefix::parse_hex(query) {
            let matches = self
                .summaries
                .iter()
                .filter(|summary| {
                    summary
                        .id
                        .as_uuid()
                        .simple()
                        .to_string()
                        .starts_with(prefix.as_str())
                })
                .cloned()
                .map(|record| FoundRecord {
                    match_kind: MatchKind::IdPrefix,
                    record,
                })
                .collect();
            return find_result(query, matches);
        }

        let normalized = query.to_lowercase();
        let matches = self
            .summaries
            .iter()
            .filter_map(|summary| {
                let filename_matches = summary
                    .path
                    .file_name()
                    .is_some_and(|name| name.to_lowercase().contains(&normalized));
                if filename_matches {
                    return Some(FoundRecord {
                        match_kind: MatchKind::Filename,
                        record: summary.clone(),
                    });
                }

                if summary.title.to_lowercase().contains(&normalized) {
                    return Some(FoundRecord {
                        match_kind: MatchKind::Title,
                        record: summary.clone(),
                    });
                }

                None
            })
            .collect();

        find_result(query, matches)
    }

    pub fn resolve_one(&self, query: &str) -> Result<FoundRecord, DadrError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(DadrError::InvalidInput(
                "record query cannot be empty".to_owned(),
            ));
        }
        let result = self.find_records(query);
        match result.resolution {
            FindResolution::None => Err(DadrError::RecordNotFound(query.to_owned())),
            FindResolution::One => result.matches.into_iter().next().ok_or_else(|| {
                DadrError::ProjectInvariant(
                    "one record resolution did not contain a match".to_owned(),
                )
            }),
            FindResolution::Many => Err(DadrError::AmbiguousRecordQuery {
                query: query.to_owned(),
                matches: result.matches.len(),
            }),
        }
    }

    /// Records matching a record link target (a short id or a full id).
    pub fn find_record_ref(&self, target: &RecordRef) -> FindRecordsResult {
        let match_kind = match target {
            RecordRef::ShortId(_) => MatchKind::IdPrefix,
            RecordRef::Full(_) => MatchKind::ExactId,
        };
        let matches = self
            .summaries
            .iter()
            .filter(|summary| target.matches(summary.id))
            .cloned()
            .map(|record| FoundRecord {
                match_kind: match_kind.clone(),
                record,
            })
            .collect();
        find_result(&target.to_string(), matches)
    }

    /// Whether a short id is used by a record, or by a file that names it but
    /// could not be parsed.
    pub fn short_id_taken(&self, short_id: &str) -> bool {
        let filename_prefix = format!("{short_id}-");
        self.scanned.iter().any(|scanned| {
            scanned
                .record
                .as_ref()
                .is_some_and(|record| record.front_matter.id.short_id() == short_id)
                || scanned
                    .path
                    .file_name()
                    .is_some_and(|name| name.starts_with(&filename_prefix))
        })
    }
}

pub(crate) fn find_result(query: &str, matches: Vec<FoundRecord>) -> FindRecordsResult {
    let resolution = match matches.len() {
        0 => FindResolution::None,
        1 => FindResolution::One,
        _ => FindResolution::Many,
    };
    FindRecordsResult {
        query: query.to_owned(),
        resolution,
        matches,
    }
}
