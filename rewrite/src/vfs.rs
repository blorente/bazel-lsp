use eyre::{eyre, Result};
use std::{collections::HashMap, path::PathBuf};

pub(crate) struct AbsPath(PathBuf);
pub(crate) type FileId = u32;
pub(crate) type FileContent = Vec<u8>;

pub(crate) struct Vfs {
    files: HashMap<FileId, FileContent>,
}

impl Vfs {
    pub(crate) fn ingest(path: &AbsPath) -> Result<FileId> {
        Err(eyre!("Vfs.ingest not implemented!"))
    }

    pub(crate) fn contents(path: FileId) -> Result<FileContent> {
        Err(eyre!("Vfs.contents not implemented!"))
    }
}
