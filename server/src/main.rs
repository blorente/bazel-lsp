use std::sync::Mutex;

use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use starlark::stdlib::global_environment;
use starlark::syntax::dialect::Dialect;

mod interpreter;
use interpreter::{BazelWorkspaceLoader, Starlark};

mod parser;
use parser::highlight;
use parser::extract_symbols;

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
        capabilities.document_highlight_provider = Some(true);
        capabilities.document_symbol_provider = Some(true);
        capabilities.definition_provider = Some(true);
        capabilities.workspace = Some(WorkspaceCapability {
            workspace_folders: Some(WorkspaceFolderCapability {
                supported: Some(true),
                change_notifications: None,
            }),
        });
        capabilities
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
        self.client
            .log_message(
                MessageType::Info,
                format!("opened file {}", params.text_document.uri),
            )
            .await;
        
        let path = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())?;
        self.documents.refresh_doc(&path);
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let path = params
            .text_document_position_params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())?;
        self.client
            .log_message(MessageType::Info, format!("Highlighting file {:?}", &path))
            .await;

        highlight(&path).map_err(|_| Error::internal_error())
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let path = params
            .text_document
            .uri
            .to_file_path()
            .map_err(|_| Error::internal_error())?;
        let resp = extract_symbols(&path)
            .map_err(|_| Error::internal_error())?;
        
        match resp {
            Some(symbols) => Ok(Some(DocumentSymbolResponse::Flat(symbols))),
            None => Ok(None)
        }
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {

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
