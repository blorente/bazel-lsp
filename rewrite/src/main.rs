use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use eyre::{Result, eyre};
use bazel::bazel::BazelExecutable;
use bazel::workspace::workspace::BazelWorkspace;
use bazel::workspace::workspaces::BazelWorkspaces;
use tower_lsp::jsonrpc::{Error, Result as LspResult};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[macro_use] extern crate maplit;

mod bazel;
use vfs::VfsHandle;

mod vfs;

#[derive(Debug)]
struct Backend {
    client: Client,
    vfs: VfsHandle,
    workspaces: BazelWorkspaces,
    bazel_exe: BazelExecutable,
    /// This is a mutex of an option because switching from no current to one current should be
    /// sync.
    current_workspace: Mutex<Option<Arc<BazelWorkspace>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        let vfs = VfsHandle::new();
        let workspaces = BazelWorkspaces::new();
        let bazel_exe = BazelExecutable::new("/opt/brew/bin/bazel");
        Backend {
            client,
            vfs,
            workspaces,
            current_workspace: Mutex::new(None),
            bazel_exe,
        }
    }

    fn capabilities() -> ServerCapabilities {
        let mut capabilities = ServerCapabilities::default();
        capabilities.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full));
        capabilities.definition_provider = Some(true);
        capabilities.workspace = Some(WorkspaceCapability {
            workspace_folders: Some(WorkspaceFolderCapability {
                supported: Some(true),
                change_notifications: None,
            }),
        });
        capabilities
    }

    fn switch_workspace(&self, root: &PathBuf) -> eyre::Result<()> {
        let workspace: Arc<BazelWorkspace> = self.workspaces.get_workspace(&self.bazel_exe, root, self.vfs.clone())?;
        let mut current = self.current_workspace.lock().map_err(|err| eyre!("Failed to lock current workspace: {}", err))?;
        *current = Some(workspace);
        Ok(())
    }
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        self.client
            .log_message(MessageType::Info, "initialized!")
            .await;
        panic!("PLease fail");
        params
            .root_uri
            .ok_or_else(|| Error::internal_error())
            .and_then(|url| url.to_file_path().map_err(|_| Error::internal_error()))
            .map(|path| self.switch_workspace(&path))?;
            //.and_then(|path| self.bazel.update_workspace(&path).map_err(|_| Error::internal_error()))?;
        Ok(InitializeResult {
            capabilities: Backend::capabilities(),
            server_info: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::Info, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        self.client.log_message(MessageType::Info, "goodbye!").await;
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())
            .expect("bad path");
        //self.update_doc(&path).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())
            .expect("bad path");
        //self.update_bazel(&path).await;
        //self.update_doc(&path).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri;
        let path = uri
            .to_file_path()
            .map_err(|_| Error::internal_error())?;
        let position = params.text_document_position_params.position;
        //let maybe_location = self.documents.locate_declaration_of_call_at(&path, position);
        //self.client
            //.log_message(
                //MessageType::Info,
                //format!("Goto Location {:#?}", &maybe_location),
            //)
            //.await;
        //if let Some(loc) = &maybe_location {
            //self.update_bazel(&loc.uri.to_file_path().expect("")).await;
        //}
        let maybe_location = None;
        Ok(maybe_location.map(|loc| {
            GotoDefinitionResponse::Scalar(loc)
        }))
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let read = tokio::io::stdin();
    let write = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend::new(client));
    Server::new(read, write)
        .interleave(messages)
        .serve(service)
        .await;

    Ok(())
}
