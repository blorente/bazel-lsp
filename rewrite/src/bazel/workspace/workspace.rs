use eyre::eyre;
use eyre::Result;
use std::path::PathBuf;

use crate::bazel::{bazel::BazelExecutable, info::BazelInfo, symbol_index::SymbolIndex};
use crate::vfs::VfsHandle;

#[derive(Debug)]
pub(crate) struct BazelWorkspace {
    info: BazelInfo,
    symbol_idx: SymbolIndex,
    vfs: VfsHandle,
}

impl BazelWorkspace {
    pub(crate) fn new(bazel: &BazelExecutable, root: &PathBuf, vfs: VfsHandle) -> Result<Self> {
        let info = BazelInfo::new(bazel, root)?;
        Ok(BazelWorkspace {
            info,
            symbol_idx: SymbolIndex::new(),
            vfs: vfs.clone(),
        })
    }
}
