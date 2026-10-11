//! Where record files go, and writing a lifecycle operation's files as one
//! unit.
//!
//! Filenames carry no meaning: a record's identity is its `id`. Dadrock names
//! a new record after its title and keeps the name when the record changes
//! folders; the short id is appended only to resolve a collision in a folder.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use camino::{Utf8Path, Utf8PathBuf};

use crate::{error::DadrError, file_store, ids::RecordId, project::utf8_to_path, text::ascii_slug};

/// The path for a new record in `dir`: `<slug>.md`, from its title, or
/// `<slug>-<short-id>.md` when the folder already has that name.
pub(super) fn new_record_path(
    dir: &Utf8Path,
    id: RecordId,
    title: &str,
) -> Result<Utf8PathBuf, DadrError> {
    let slug = ascii_slug(title, 80, "untitled");
    let path = dir.join(format!("{slug}.md"));
    if !path_taken(&path) {
        return Ok(path);
    }
    let path = dir.join(format!("{slug}-{}.md", id.short_id()));
    if !path_taken(&path) {
        return Ok(path);
    }
    Err(DadrError::OverwriteCollision { path })
}

/// A record that a lifecycle operation moves to a folder.
pub(super) struct Relocation<'a> {
    pub(super) from: &'a Utf8Path,
    pub(super) id: RecordId,
    pub(super) dir: Utf8PathBuf,
}

/// Where each record of one operation lands: its folder joined with its
/// current filename, or `<stem>-<short-id>.md` when that name is taken. A
/// record already at that path stays in place.
///
/// A path that another record of the same operation leaves counts as free, so
/// a successor that keeps its predecessor's title takes the predecessor's
/// filename in the ledger. Two records never land on the same path. When even
/// the suffixed name is taken, nothing is planned.
pub(super) fn relocate(moves: &[Relocation<'_>]) -> Result<Vec<Utf8PathBuf>, DadrError> {
    let kept = moves
        .iter()
        .map(|record| Ok(record.dir.join(record_file_name(record.from)?)))
        .collect::<Result<Vec<_>, DadrError>>()?;
    let vacated: BTreeSet<&Utf8Path> = moves
        .iter()
        .zip(&kept)
        .filter(|(record, kept)| record.from != kept.as_path())
        .map(|(record, _)| record.from)
        .collect();
    // Records that stay in place hold their paths from the start.
    let mut claimed: BTreeSet<Utf8PathBuf> = moves
        .iter()
        .zip(&kept)
        .filter(|(record, kept)| record.from == kept.as_path())
        .map(|(_, kept)| kept.clone())
        .collect();
    let free = |path: &Utf8Path, claimed: &BTreeSet<Utf8PathBuf>| {
        !claimed.contains(path) && (vacated.contains(path) || !path_taken(path))
    };

    let mut targets = Vec::with_capacity(moves.len());
    for (record, kept) in moves.iter().zip(kept) {
        let target = if record.from == kept.as_path() || free(&kept, &claimed) {
            kept
        } else {
            match with_short_id(&kept, record.id) {
                Some(suffixed) if free(&suffixed, &claimed) => suffixed,
                Some(suffixed) => return Err(DadrError::OverwriteCollision { path: suffixed }),
                None => return Err(DadrError::OverwriteCollision { path: kept }),
            }
        };
        claimed.insert(target.clone());
        targets.push(target);
    }
    Ok(targets)
}

/// `<stem>-<short-id>.<ext>` beside `path`, unless the stem already ends with
/// the short id.
fn with_short_id(path: &Utf8Path, id: RecordId) -> Option<Utf8PathBuf> {
    let stem = path.file_stem()?;
    let suffix = format!("-{}", id.short_id());
    if stem.ends_with(&suffix) {
        return None;
    }
    let extension = path.extension().unwrap_or("md");
    Some(path.with_file_name(format!("{stem}{suffix}.{extension}")))
}

pub(super) fn record_file_name(path: &Utf8Path) -> Result<&str, DadrError> {
    path.file_name()
        .ok_or_else(|| DadrError::ProjectInvariant(format!("record path `{path}` has no filename")))
}

/// Whether anything, even a dangling symlink, is at `path`.
fn path_taken(path: &Utf8Path) -> bool {
    fs::symlink_metadata(utf8_to_path(path)).is_ok()
}

/// One record file, rewritten in place or moved to another folder.
pub(super) struct RecordWrite {
    pub(super) from: Utf8PathBuf,
    pub(super) to: Utf8PathBuf,
    original: String,
    updated: String,
}

impl RecordWrite {
    pub(super) fn read(
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

/// Apply every write or none.
///
/// A move can land on a path that another move of the same operation leaves,
/// such as a successor taking its predecessor's filename in the ledger, so the
/// order matters:
///
/// 1. Every target is staged beside its final path before anything changes.
/// 2. Targets are committed (renamed into place) so that a write whose target
///    is another move's source commits only after that move's own target: the
///    record it replaces is already at its new path, and in memory.
/// 3. The sources of moves are removed last, except a source that is another
///    write's target, which now holds that write's record.
///
/// On an I/O error, every source gets its original content back first; then
/// each moved copy at a path that held no record before is removed.
pub(super) fn apply_record_writes(writes: &[RecordWrite]) -> Result<(), DadrError> {
    let order = commit_order(writes)?;

    let mut staged = Vec::with_capacity(writes.len());
    for write in writes {
        step()?;
        staged.push(Some(file_store::stage_replace(&write.to, &write.updated)?));
    }

    let mut attempted = vec![false; writes.len()];
    let applied = (|| {
        for &index in &order {
            attempted[index] = true;
            step()?;
            if let Some(staged) = staged[index].take() {
                staged.commit()?;
            }
        }
        let targets: BTreeSet<&Utf8Path> = writes.iter().map(|write| write.to.as_path()).collect();
        for write in writes {
            if write.moves() && !targets.contains(write.from.as_path()) {
                step()?;
                file_store::remove_file(&write.from)?;
            }
        }
        Ok::<_, DadrError>(())
    })();
    if let Err(error) = applied {
        if !crashed() {
            restore_record_writes(writes, &attempted);
        }
        return Err(error);
    }
    Ok(())
}

/// The order to commit writes in. Checks, before anything is written, that no
/// two writes share a source or a target, and that each move lands on a free
/// path or on one another move leaves.
fn commit_order(writes: &[RecordWrite]) -> Result<Vec<usize>, DadrError> {
    let mut targets = BTreeSet::new();
    let mut sources = BTreeMap::new();
    for (index, write) in writes.iter().enumerate() {
        if !targets.insert(write.to.as_path()) {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "two records would be written to `{}`",
                write.to
            )));
        }
        if sources.insert(write.from.as_path(), index).is_some() {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "`{}` would be written twice",
                write.from
            )));
        }
    }
    // The move, if any, that leaves a write's target.
    let vacated_by = |write: &RecordWrite| {
        sources
            .get(write.to.as_path())
            .copied()
            .filter(|&other| writes[other].moves())
    };
    for write in writes.iter().filter(|write| write.moves()) {
        if vacated_by(write).is_none() && path_taken(&write.to) {
            return Err(DadrError::OverwriteCollision {
                path: write.to.clone(),
            });
        }
    }

    // Each path is at most one write's source and one write's target, so the
    // writes form chains; each one commits after the move it depends on.
    let mut order = Vec::with_capacity(writes.len());
    let mut committed = vec![false; writes.len()];
    while order.len() < writes.len() {
        let before = order.len();
        for (index, write) in writes.iter().enumerate() {
            if !committed[index] && vacated_by(write).is_none_or(|other| committed[other]) {
                committed[index] = true;
                order.push(index);
            }
        }
        if order.len() == before {
            return Err(DadrError::InvalidLifecycleTransition(
                "records cannot trade places in one operation".to_owned(),
            ));
        }
    }
    Ok(order)
}

fn restore_record_writes(writes: &[RecordWrite], attempted: &[bool]) {
    // Bring every original back before removing any moved copy, so no record
    // is ever missing from disk. A source that another write replaced gets its
    // own record back.
    for write in writes {
        let intact = fs::read_to_string(utf8_to_path(&write.from))
            .is_ok_and(|content| content == write.original);
        if !intact {
            let _ = file_store::write_replace(&write.from, &write.original);
        }
    }
    // Every target was free or a source before the operation, so a moved copy
    // at a path that is no source is one this operation wrote.
    let sources: BTreeSet<&Utf8Path> = writes.iter().map(|write| write.from.as_path()).collect();
    for (write, attempted) in writes.iter().zip(attempted) {
        if *attempted
            && write.moves()
            && !sources.contains(write.to.as_path())
            && path_taken(&write.to)
        {
            let _ = file_store::remove_file(&write.to);
        }
    }
}

#[cfg(test)]
thread_local! {
    /// The number of file steps `apply_record_writes` takes before an injected
    /// failure; `None` never fails.
    static FAIL_AFTER_STEPS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Whether an injected failure stops like a killed process, without
    /// restoring anything.
    static CRASH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether a failure leaves the files as they are, for tests that check what
/// a killed process would leave on disk.
fn crashed() -> bool {
    #[cfg(test)]
    let crashed = CRASH.with(std::cell::Cell::get);
    #[cfg(not(test))]
    let crashed = false;
    crashed
}

/// One file step of `apply_record_writes`, where tests inject failures.
fn step() -> Result<(), DadrError> {
    #[cfg(test)]
    {
        let fail = FAIL_AFTER_STEPS.with(|steps| match steps.get() {
            Some(0) => {
                steps.set(None);
                true
            }
            Some(left) => {
                steps.set(Some(left - 1));
                false
            }
            None => false,
        });
        if fail {
            return Err(std::io::Error::other("injected failure").into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir {
        root: Utf8PathBuf,
    }

    impl Dir {
        fn new(label: &str) -> Self {
            let root = Utf8PathBuf::from_path_buf(
                std::env::temp_dir().join(format!("dadrock-writes-{label}-{}", RecordId::new_v7())),
            )
            .unwrap();
            for folder in ["inbox", "ledger", "archive"] {
                fs::create_dir_all(root.join(folder)).unwrap();
            }
            Self { root }
        }

        fn path(&self, relative: &str) -> Utf8PathBuf {
            self.root.join(relative)
        }

        fn write(&self, relative: &str, content: &str) -> Utf8PathBuf {
            let path = self.path(relative);
            fs::write(&path, content).unwrap();
            path
        }

        /// Every file under the root, by relative path, with its content.
        fn snapshot(&self) -> BTreeMap<String, String> {
            let mut files = BTreeMap::new();
            let mut dirs = vec![self.root.clone()];
            while let Some(dir) = dirs.pop() {
                for entry in dir.read_dir_utf8().unwrap() {
                    let path = entry.unwrap().into_path();
                    if path.is_dir() {
                        dirs.push(path);
                    } else {
                        let relative = path.strip_prefix(&self.root).unwrap().to_string();
                        files.insert(relative, fs::read_to_string(&path).unwrap());
                    }
                }
            }
            files
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn id(n: u8) -> RecordId {
        RecordId::parse(&format!("01938b8e-4d7{n}-7abc-8def-0123456789ab")).unwrap()
    }

    fn write(from: &Utf8Path, to: Utf8PathBuf, label: &str) -> RecordWrite {
        RecordWrite::read(from, to, |content| Ok(format!("{content}{label}\n"))).unwrap()
    }

    fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(path, content)| ((*path).to_owned(), (*content).to_owned()))
            .collect()
    }

    /// Applies the writes `build` returns with a failure injected at every
    /// step in turn, checking that each failure leaves every file exactly as
    /// it was, then applies them for real.
    fn apply_with_every_failure(dir: &Dir, build: impl Fn() -> Vec<RecordWrite>) {
        let before = dir.snapshot();
        let mut failures = 0;
        loop {
            FAIL_AFTER_STEPS.with(|steps| steps.set(Some(failures)));
            let result = apply_record_writes(&build());
            let injected = FAIL_AFTER_STEPS.with(|steps| steps.replace(None)).is_none();
            if !injected {
                result.unwrap();
                break;
            }
            assert!(result.is_err(), "step {failures} should have failed");
            assert_eq!(dir.snapshot(), before, "after a failure at step {failures}");
            failures += 1;
        }
        assert!(failures > 0);
    }

    /// Stops the writes of a fresh scenario at every step in turn, like a
    /// killed process, and checks that each record is still on disk, as it
    /// was or as it was to become.
    fn crash_at_every_step(label: &str, scenario: impl Fn(&Dir) -> Vec<RecordWrite>) {
        let mut crashes = 0;
        loop {
            let dir = Dir::new(label);
            let writes = scenario(&dir);
            CRASH.with(|crash| crash.set(true));
            FAIL_AFTER_STEPS.with(|steps| steps.set(Some(crashes)));
            let result = apply_record_writes(&writes);
            CRASH.with(|crash| crash.set(false));
            let injected = FAIL_AFTER_STEPS.with(|steps| steps.replace(None)).is_none();
            if !injected {
                result.unwrap();
                break;
            }
            let on_disk: Vec<String> = dir.snapshot().into_values().collect();
            for write in &writes {
                assert!(
                    on_disk.contains(&write.original) || on_disk.contains(&write.updated),
                    "`{}` is missing from disk after a crash at step {crashes}",
                    write.from
                );
            }
            crashes += 1;
        }
        assert!(crashes > 0);
    }

    #[test]
    fn new_records_take_the_slug_then_the_short_id() {
        let dir = Dir::new("new");
        let inbox = dir.path("inbox");
        let first = new_record_path(&inbox, id(0), "Drop missing rows!").unwrap();
        assert_eq!(first, inbox.join("drop-missing-rows.md"));
        fs::write(&first, "").unwrap();

        let second = new_record_path(&inbox, id(1), "Drop missing rows").unwrap();
        assert_eq!(
            second,
            inbox.join(format!("drop-missing-rows-{}.md", id(1).short_id()))
        );
        fs::write(&second, "").unwrap();

        let error = new_record_path(&inbox, id(1), "Drop missing rows").unwrap_err();
        assert!(matches!(error, DadrError::OverwriteCollision { path } if path == second));
        assert_eq!(
            new_record_path(&inbox, id(2), "???").unwrap(),
            inbox.join("untitled.md")
        );
    }

    #[test]
    fn moves_keep_the_filename_and_free_vacated_paths() {
        let dir = Dir::new("relocate");
        let successor = dir.write("inbox/outcome.md", "");
        let predecessor = dir.write("ledger/outcome.md", "");
        dir.write("archive/outcome.md", "");
        let in_place = dir.write("archive/kept.md", "");

        let targets = relocate(&[
            Relocation {
                from: &successor,
                id: id(1),
                dir: dir.path("ledger"),
            },
            Relocation {
                from: &predecessor,
                id: id(2),
                dir: dir.path("archive"),
            },
            Relocation {
                from: &in_place,
                id: id(3),
                dir: dir.path("archive"),
            },
        ])
        .unwrap();
        assert_eq!(
            targets,
            [
                dir.path("ledger/outcome.md"),
                dir.path(&format!("archive/outcome-{}.md", id(2).short_id())),
                in_place.clone(),
            ]
        );
    }

    #[test]
    fn two_records_moving_to_one_name_get_distinct_paths() {
        let dir = Dir::new("claimed");
        let left = dir.write("inbox/rule.md", "");
        fs::create_dir_all(dir.path("inbox/nested")).unwrap();
        let right = dir.write("inbox/nested/rule.md", "");
        let targets = relocate(&[
            Relocation {
                from: &left,
                id: id(1),
                dir: dir.path("ledger"),
            },
            Relocation {
                from: &right,
                id: id(2),
                dir: dir.path("ledger"),
            },
        ])
        .unwrap();
        assert_eq!(
            targets,
            [
                dir.path("ledger/rule.md"),
                dir.path(&format!("ledger/rule-{}.md", id(2).short_id())),
            ]
        );
    }

    #[test]
    fn a_taken_suffixed_name_refuses_the_move() {
        let dir = Dir::new("taken");
        let short = id(1).short_id();
        let candidate = dir.write(&format!("inbox/rule-{short}.md"), "");
        dir.write(&format!("ledger/rule-{short}.md"), "");
        let error = relocate(&[Relocation {
            from: &candidate,
            id: id(1),
            dir: dir.path("ledger"),
        }])
        .unwrap_err();
        assert!(
            matches!(error, DadrError::OverwriteCollision { .. }),
            "{error}"
        );

        let other = dir.write("inbox/other.md", "");
        dir.write("ledger/other.md", "");
        dir.write(&format!("ledger/other-{short}.md"), "");
        let error = relocate(&[Relocation {
            from: &other,
            id: id(1),
            dir: dir.path("ledger"),
        }])
        .unwrap_err();
        assert!(
            matches!(error, DadrError::OverwriteCollision { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_successor_takes_its_predecessors_path_without_losing_either() {
        let dir = Dir::new("same-title");
        let successor = dir.write("inbox/outcome.md", "successor\n");
        let predecessor = dir.write("ledger/outcome.md", "predecessor\n");
        // The successor comes first, as accepting writes it.
        let build = || {
            vec![
                write(&successor, dir.path("ledger/outcome.md"), "accepted"),
                write(&predecessor, dir.path("archive/outcome.md"), "superseded"),
            ]
        };
        apply_with_every_failure(&dir, build);
        assert_eq!(
            dir.snapshot(),
            files(&[
                ("ledger/outcome.md", "successor\naccepted\n"),
                ("archive/outcome.md", "predecessor\nsuperseded\n"),
            ])
        );
    }

    #[test]
    fn a_crash_at_any_step_leaves_every_record_on_disk() {
        crash_at_every_step("crash-same-title", |dir| {
            let successor = dir.write("inbox/outcome.md", "successor\n");
            let predecessor = dir.write("ledger/outcome.md", "predecessor\n");
            vec![
                write(&successor, dir.path("ledger/outcome.md"), "accepted"),
                write(&predecessor, dir.path("archive/outcome.md"), "superseded"),
            ]
        });
        crash_at_every_step("crash-chain", |dir| {
            let candidate = dir.write("inbox/rule.md", "candidate\n");
            let accepted = dir.write("ledger/rule.md", "accepted\n");
            fs::create_dir_all(dir.path("ledger/old")).unwrap();
            let older = dir.write("ledger/old/rule.md", "older\n");
            vec![
                write(&candidate, accepted.clone(), "a"),
                write(&accepted, older.clone(), "b"),
                write(&older, dir.path("archive/rule.md"), "c"),
            ]
        });
    }

    #[test]
    fn a_chain_of_vacated_paths_commits_from_its_end() {
        let dir = Dir::new("chain");
        let candidate = dir.write("inbox/rule.md", "candidate\n");
        let accepted = dir.write("ledger/rule.md", "accepted\n");
        let nested = dir.path("ledger/old");
        fs::create_dir_all(&nested).unwrap();
        let older = dir.write("ledger/old/rule.md", "older\n");
        // inbox/rule.md -> ledger/rule.md -> ledger/old/rule.md -> archive/rule.md
        let build = || {
            vec![
                write(&candidate, accepted.clone(), "a"),
                write(&accepted, older.clone(), "b"),
                write(&older, dir.path("archive/rule.md"), "c"),
            ]
        };
        apply_with_every_failure(&dir, build);
        assert_eq!(
            dir.snapshot(),
            files(&[
                ("ledger/rule.md", "candidate\na\n"),
                ("ledger/old/rule.md", "accepted\nb\n"),
                ("archive/rule.md", "older\nc\n"),
            ])
        );
    }

    #[test]
    fn in_place_writes_and_moves_restore_together() {
        let dir = Dir::new("in-place");
        let candidate = dir.write("inbox/a.md", "a\n");
        let rewritten = dir.write("archive/b.md", "b\n");
        let build = || {
            vec![
                write(&candidate, dir.path("ledger/a.md"), "moved"),
                write(&rewritten, rewritten.clone(), "rewritten"),
            ]
        };
        apply_with_every_failure(&dir, build);
        assert_eq!(
            dir.snapshot(),
            files(&[
                ("ledger/a.md", "a\nmoved\n"),
                ("archive/b.md", "b\nrewritten\n"),
            ])
        );
    }

    #[test]
    fn writes_that_cannot_be_ordered_or_placed_change_nothing() {
        let dir = Dir::new("refused");
        let left = dir.write("inbox/left.md", "left\n");
        let right = dir.write("ledger/right.md", "right\n");
        let taken = dir.write("archive/taken.md", "taken\n");
        let before = dir.snapshot();

        // Two records trading places.
        let error = apply_record_writes(&[
            write(&left, right.clone(), "x"),
            write(&right, left.clone(), "y"),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("trade places"), "{error}");

        // A target that holds a record no write moves away.
        let error = apply_record_writes(&[write(&left, taken.clone(), "x")]).unwrap_err();
        assert!(
            matches!(error, DadrError::OverwriteCollision { .. }),
            "{error}"
        );

        // Two writes to one target.
        let target = dir.path("ledger/same.md");
        let error = apply_record_writes(&[
            write(&left, target.clone(), "x"),
            write(&right, target, "y"),
        ])
        .unwrap_err();
        assert!(error.to_string().contains("two records"), "{error}");

        assert_eq!(dir.snapshot(), before);
    }
}
