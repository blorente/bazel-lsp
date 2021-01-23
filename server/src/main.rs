use std::path::PathBuf;
use std::sync::Mutex;

use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use starlark::stdlib::global_environment;
use starlark::syntax::dialect::Dialect;

mod interpreter;
use interpreter::{BazelWorkspaceLoader, Starlark};

mod index;
use index::Documents;

#[derive(Debug)]
struct Backend {
    client: Client,
    loader: BazelWorkspaceLoader,
    documents: Documents,
}

impl Backend {
    fn new(client: Client) -> Self {
        Backend {
            client,
            loader: BazelWorkspaceLoader { workspace: None },
            documents: Documents::default(),
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

    async fn update_doc(&self, doc: &PathBuf) {
        self.client
            .log_message(MessageType::Log, format!("opened file {:?}", doc))
            .await;

        self.documents.refresh_doc(doc);
        self.client
            .log_message(
                MessageType::Log,
                format!("index is now {:#?}", self.documents),
            )
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::Info, "initialized!")
            .await;
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

    async fn shutdown(&self) -> Result<()> {
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
        self.update_doc(&path).await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())
            .expect("bad path");
        self.update_doc(&path).await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri;
        let path = uri
            .to_file_path()
            .map_err(|_| Error::internal_error())?;
        let position = params.text_document_position_params.position;
        let index = self.documents.get_doc(&path).expect("Index missing");
        let maybe_declaration = index
            .call_at(position);

        self.client
            .log_message(
                MessageType::Log,
                format!("Got call {:#?}", &maybe_declaration),
            )
            .await;
        let maybe_declaration = maybe_declaration
            .and_then(|call| index.declaration_of(&call));
        self.client
            .log_message(
                MessageType::Log,
                format!("Goto Declaration {:#?}", &maybe_declaration),
            )
            .await;
        // TODO Goto calls defined in the same file
        Ok(maybe_declaration.map(|decl| {
            GotoDefinitionResponse::Scalar(decl.lsp_location(&uri))
        }))
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    // let mut listener = tokio::net::TcpListener::bind("127.0.0.1:9274").await?;
    // let (stream, _) = listener.accept().await?;
    // let (read, write) = tokio::io::split(stream);
    let read = tokio::io::stdin();
    let write = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend::new(client));
    Server::new(read, write)
        .interleave(messages)
        .serve(service)
        .await;

    Ok(())
}
