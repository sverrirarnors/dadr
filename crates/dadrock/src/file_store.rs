use std::{fs, io::Write, process};

use camino::{Utf8Path, Utf8PathBuf};

use crate::{error::DadrError, ids::RecordId, project::utf8_to_path};

pub(crate) fn write_new(path: &Utf8Path, content: &str) -> Result<(), DadrError> {
    if path.exists() {
        return Err(DadrError::OverwriteCollision {
            path: path.to_owned(),
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(utf8_to_path(parent))?;
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(utf8_to_path(path))?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    sync_parent_dir(path)?;
    Ok(())
}

pub(crate) fn write_replace(path: &Utf8Path, content: &str) -> Result<(), DadrError> {
    stage_replace(path, content)?.commit()
}

pub(crate) struct StagedReplace {
    target: Utf8PathBuf,
    tmp: Utf8PathBuf,
    committed: bool,
}

impl StagedReplace {
    pub(crate) fn commit(mut self) -> Result<(), DadrError> {
        fs::rename(utf8_to_path(&self.tmp), utf8_to_path(&self.target))?;
        self.committed = true;
        sync_parent_dir(&self.target)
    }
}

impl Drop for StagedReplace {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(utf8_to_path(&self.tmp));
        }
    }
}

pub(crate) fn stage_replace(path: &Utf8Path, content: &str) -> Result<StagedReplace, DadrError> {
    let tmp = temporary_replace_path(path);
    write_new(&tmp, content)?;
    Ok(StagedReplace {
        target: path.to_owned(),
        tmp,
        committed: false,
    })
}

pub(crate) fn remove_file(path: &Utf8Path) -> Result<(), DadrError> {
    fs::remove_file(utf8_to_path(path))?;
    sync_parent_dir(path)
}

pub(crate) struct RemoveFileFailure {
    pub error: DadrError,
    pub unlinked: bool,
}

pub(crate) fn remove_file_with_unlink_status(path: &Utf8Path) -> Result<(), RemoveFileFailure> {
    fs::remove_file(utf8_to_path(path)).map_err(|err| RemoveFileFailure {
        error: DadrError::Io(err),
        unlinked: false,
    })?;
    sync_parent_dir(path).map_err(|error| RemoveFileFailure {
        error,
        unlinked: true,
    })
}

pub(crate) fn sync_parent_dir(path: &Utf8Path) -> Result<(), DadrError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    sync_dir(parent)
}

pub(crate) fn sync_dir(path: &Utf8Path) -> Result<(), DadrError> {
    let dir = fs::File::open(utf8_to_path(path))?;
    dir.sync_all()?;
    Ok(())
}

fn temporary_replace_path(path: &Utf8Path) -> Utf8PathBuf {
    let suffix = format!("tmp-{}-{}", process::id(), RecordId::new_v7());
    match path.extension() {
        Some(extension) => path.with_extension(format!("{extension}.{suffix}")),
        None => path.with_extension(suffix),
    }
}
