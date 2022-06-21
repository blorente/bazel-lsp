use eyre::eyre;
use eyre::Result;
use std::{collections::HashMap, path::PathBuf, sync::Arc, sync::RwLock};

use crate::{
    bazel::{bazel::BazelExecutable, info::BazelInfo, workspace::workspace::BazelWorkspace},
    vfs::VfsHandle,
};

#[derive(Debug)]
pub(crate) struct BazelWorkspaces {
    workspaces: RwLock<HashMap<PathBuf, Arc<BazelWorkspace>>>,
}
impl BazelWorkspaces {
    pub(crate) fn new() -> Self {
        BazelWorkspaces {
            workspaces: RwLock::new(HashMap::new()),
        }
    }

    fn register_workspace(
        &self,
        bazel: &BazelExecutable,
        root: &PathBuf,
        vfs: VfsHandle,
    ) -> Result<Arc<BazelWorkspace>> {
        let mut workspaces = self
            .workspaces
            .write()
            .map_err(|err| eyre!("Error when locking workspaces: {}", err))?;
        let new_ws = Arc::new(BazelWorkspace::new(bazel, root, vfs)?);
        workspaces.insert(root.clone(), new_ws.clone());
        Ok(new_ws)
    }

    pub(crate) fn get_workspace(
        &self,
        bazel: &BazelExecutable,
        root: &PathBuf,
        vfs: VfsHandle,
    ) -> Result<Arc<BazelWorkspace>> {
        {
            // We open a new scope so that the read guard is dropped when we just read.
            let workspaces = self
                .workspaces
                .read()
                .map_err(|err| eyre!("Error when locking workspaces for reading: {}", err))?;
            if let Some(ws) = workspaces.get(root) {
                return Ok(ws.clone());
            }
        }
        self.register_workspace(bazel, root, vfs)
    }
}
