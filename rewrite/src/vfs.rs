use eyre::{eyre, Result};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

pub(crate) struct AbsPath(PathBuf);
pub(crate) type FileId = u32;
pub(crate) type FileContent = Vec<u8>;

/// Cheap to copy implementation of Vfs.
/// Supposed to be passed everywhere.
/// TODO: Consider moving new() to Vfs, so that Vfs::new() -> VfsHandle
#[derive(Clone, Debug)]
pub(crate) struct VfsHandle {
    internal: Arc<RwLock<VfsInternal>>,
}
impl VfsHandle {
    pub(crate) fn new() -> Self {
        VfsHandle {
            internal: Arc::new(RwLock::new(VfsInternal {
                files: HashMap::new(),
            })),
        }
    }
    pub(crate) fn ingest(path: &AbsPath) -> Result<FileId> {
        Err(eyre!("Vfs.ingest not implemented!"))
    }

    pub(crate) fn contents(path: FileId) -> Result<FileContent> {
        Err(eyre!("Vfs.contents not implemented!"))
    }
}

#[derive(Debug)]
struct VfsInternal {
    files: HashMap<FileId, FileContent>,
}
