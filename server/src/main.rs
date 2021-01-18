

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.client.log_message(MessageType::Info, "initialized!").await;
        
        Ok(InitializeResult::default())
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
}

#[tokio::main]
async fn main() -> tokio::io::Result<()>  {
    // let mut listener = tokio::net::TcpListener::bind("127.0.0.1:9274").await?;
    // let (stream, _) = listener.accept().await?;
    // let (read, write) = tokio::io::split(stream);
    let read = tokio::io::stdin();
    let write = tokio::io::stdout();

    let (service, messages) = LspService::new(|client| Backend { client });
    Server::new(read, write)
        .interleave(messages)
        .serve(service)
        .await;

    Ok(())
}