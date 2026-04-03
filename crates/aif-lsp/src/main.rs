use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod diagnostics;
mod semantic_tokens;
mod symbols;

struct AifLanguageServer {
    client: Client,
}

impl AifLanguageServer {
    fn new(client: Client) -> Self {
        Self { client }
    }

    async fn on_change(&self, uri: Url, text: &str) {
        let diags = diagnostics::compute_diagnostics(text);
        self.client
            .publish_diagnostics(uri, diags, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for AifLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensOptions {
                        legend: semantic_tokens::legend(),
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: None,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    }
                    .into(),
                ),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "aif-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "AIF Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.on_change(params.text_document.uri, &change.text).await;
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        // We need the document text but tower-lsp doesn't store it for us.
        // In a full implementation we'd maintain a document store.
        // For now, return empty tokens — diagnostics work on didOpen/didChange.
        let _ = params;
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // Requires document store to resolve references. Placeholder for now.
        let _ = params;
        Ok(None)
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(AifLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
