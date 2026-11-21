use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::{DocumentManager, semantic_tokens};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct Backend {
    client: Client,
    docs: DocumentManager,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: Default::default(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "reqlsp".into(),
                version: Some(VERSION.into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![" ".into(), "[".into(), ":".into()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.docs.insert(&doc.uri, &doc.text, doc.version).await;
        let diags = self.docs.diagnostics(&doc.uri).await;
        self.client
            .publish_diagnostics(doc.uri, diags, Some(doc.version))
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let doc = params.text_document;
        for change in params.content_changes {
            match change.range {
                Some(_) => {
                    self.client
                        .log_message(MessageType::WARNING, "range edits not yet supported")
                        .await;
                }
                None => {
                    self.docs.insert(&doc.uri, &change.text, doc.version).await;
                }
            }
        }
        let diags = self.docs.diagnostics(&doc.uri).await;
        self.client
            .publish_diagnostics(doc.uri, diags, Some(doc.version))
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let symbols = self.docs.symbols(&params.text_document.uri).await;
        Ok(symbols.map(DocumentSymbolResponse::Nested))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let params = params.text_document_position_params;
        Ok(self
            .docs
            .hover(&params.text_document.uri, params.position)
            .await)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position;
        let items = self
            .docs
            .completion(&position.text_document.uri, position.position)
            .await;
        Ok(items.map(CompletionResponse::Array))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let tokens = self.docs.semantic_tokens(&params.text_document.uri).await;
        Ok(tokens.map(SemanticTokensResult::Tokens))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let params = params.text_document_position_params;
        let location = self
            .docs
            .definition(&params.text_document.uri, params.position)
            .await;
        Ok(location.map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let position = params.text_document_position;
        Ok(self
            .docs
            .references(
                &position.text_document.uri,
                position.position,
                params.context.include_declaration,
            )
            .await)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        Ok(self.docs.format(&params.text_document.uri).await)
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let doc = params.text_document;
        self.docs.remove(&doc.uri).await;
        self.client
            .publish_diagnostics(doc.uri, Vec::new(), None)
            .await;
    }
}
