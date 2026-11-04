use itertools::MultiPeek;

use crate::{
    node::{
        self,
        entity::{
            self, Attribute, Idx, Meta, Node, RequirementVariant, TraceabilityKind,
            TraceabilityLink, Variant,
        },
        path::Path,
        reference,
        tree::{self},
    },
    syntax::{
        location::Span,
        parser::{SpansDraft, expression, markup},
        tokenizer::entity::{Keyword, TokenIterator, TokenVariant},
    },
};

impl<'b> From<&'b mut Draft<'_>> for expression::Draft<'b> {
    fn from(value: &'b mut Draft<'_>) -> Self {
        expression::Draft::new(
            &mut value.tree.expressions,
            &mut value.tree.identifiers,
            &mut value.spans.expressions,
        )
    }
}

pub(super) struct Draft<'a> {
    tree: &'a mut tree::Draft,
    spans: &'a mut SpansDraft,
}

impl<'a> Draft<'a> {
    pub fn new(tree: &'a mut tree::Draft, spans: &'a mut SpansDraft) -> Self {
        Self { tree, spans }
    }

    pub fn push_node(&mut self, node: Node, span: Span) -> Idx {
        let idx = self.tree.entities.len();
        self.tree.entities.push(node);
        self.spans.entities.push(span);
        idx.into()
    }

    pub fn push_reference(&mut self, path: Path) -> reference::Idx {
        let idx = self.tree.references.len();
        self.tree.references.push(path);
        idx.into()
    }
}

// TODO: Use peek_nth to avoid reset_peek
pub(super) fn parse_tokens(
    tokens: &mut MultiPeek<TokenIterator>,
    draft: &mut Draft,
) -> Result<Idx, Span> {
    use Keyword::*;
    use TokenVariant::*;

    let mut meta = Meta::default();

    while let Some(tok) = tokens.next() {
        match tok.variant {
            Markup(comment) => {
                let markup = markup::parse(&comment.to_string(), &mut draft.tree.references);
                meta.comment = Some(markup);
            }

            Tag(id, value) => {
                let tag = entity::Tag::new(id.to_string(), value.map(String::from));
                meta.tags.push(tag);
            }

            Reserved(Part) => {
                let mut prt = node::entity::Part::default();

                meta.label = parse_label(tokens)?;
                prt.imports = parse_imports(tokens, draft)?;
                prt.children = parse_children(tokens, &Part, draft)?;
                let node = Node::new(meta, Variant::Part(prt));
                let idx = draft.push_node(node, tok.span);
                return Ok(idx.into());
            }

            Reserved(Package) => {
                let mut pkg = node::entity::Package::default();

                meta.label = parse_label(tokens)?;
                pkg.imports = parse_imports(tokens, draft)?;
                pkg.children = parse_children(tokens, &Package, draft)?;
                let node = Node::new(meta, Variant::Package(pkg));
                let idx = draft.push_node(node, tok.span);
                return Ok(idx.into());
            }

            Reserved(Let) => {
                let label = parse_label(tokens)?;
                let token = tokens.next().ok_or(Span::default())?;
                let domain = match token.variant {
                    TokenVariant::Reserved(Keyword::In) => {
                        let token = tokens.next().ok_or(tok.span)?;

                        match token.variant {
                            TokenVariant::Expression(expr) => {
                                let draft = expression::Draft::from(&mut *draft);
                                expression::parse(expr, &token.span.start, draft)?
                            }

                            _ => return Err(token.span),
                        }
                    }
                    _ => return Err(token.span),
                };

                let mut atr = Attribute::new(domain);

                meta.label = label;
                let token = tokens.peek().ok_or(tok.span)?;
                let span = token.span.clone();
                atr.unit = match token.variant {
                    TokenVariant::Unit(expr) => {
                        tokens.next();
                        Some(expr.to_string())
                    }

                    _ => None,
                };
                tokens.reset_peek();

                let node = Node::new(meta, Variant::Attribute(atr));
                let idx = draft.push_node(node, span);

                return Ok(idx);
            }

            Reserved(Keyword::Type) => {
                use TokenVariant::*;
                let mut typ = node::entity::Type::default();
                meta.label = parse_label(tokens)?;
                match tokens.next() {
                    None => return Err(Span::default()),
                    Some(tok) if let Reserved(Has) = tok.variant => {}
                    Some(tok) => return Err(tok.span),
                }

                typ.members = match tokens.next() {
                    None => return Err(Span::default()),
                    Some(tok) if let Type(expr) = tok.variant => parse_type(expr)?,
                    Some(tok) => return Err(tok.span),
                };

                match tokens.next() {
                    Some(tok) if let Reserved(Keyword::Type) = tok.variant => {}
                    _ => return Err(tok.span),
                }

                let node = Node::new(meta, Variant::Type(typ));
                let idx = draft.push_node(node, tok.span);
                return Ok(idx);
            }

            Reserved(Requirement) => {
                use TokenVariant::*;
                let mut req = node::entity::Requirement::default();
                meta.label = parse_label(tokens)?;
                while let Some(token) = tokens.next() {
                    match token.variant {
                        Reserved(Keyword::Is) => {
                            break;
                        }

                        Reserved(Keyword::Top) => req.inhibitor.top = true,

                        Reserved(Keyword::Bottom) => req.inhibitor.bottom = true,

                        Reserved(Keyword::On) => {
                            req.restriction = match parse_identifier(tokens) {
                                Ok(ident) => Some(draft.push_reference(ident.into())),
                                Err(span) => return Err(span),
                            };
                        }

                        Reserved(Keyword::Via) => {
                            req.allocation = match parse_identifier(tokens) {
                                Ok(ident) => Some(draft.push_reference(ident.into())),
                                Err(span) => return Err(span),
                            };
                        }

                        Reserved(ref k) if TraceabilityKind::is_traceability_keyword(&k) => {
                            let kind = TraceabilityKind::try_from(k).unwrap();
                            let target = match parse_identifier(tokens) {
                                Ok(ident) => draft.push_reference(ident.into()),
                                Err(span) => return Err(span),
                            };
                            let link = TraceabilityLink::new(kind, target);
                            req.traceability.push(link);
                        }

                        _ => return Err(token.span),
                    };
                }

                let token = tokens.next().ok_or(Span::default())?;

                use RequirementVariant::*;
                req.variant = match token.variant {
                    Markup(expr) => {
                        Informal(markup::parse(&expr.to_string(), &mut draft.tree.references))
                    }
                    Expression(expr) => {
                        let draft = expression::Draft::from(&mut *draft);
                        Formal(expression::parse(expr, &token.span.start, draft)?)
                    }
                    _ => return Err(token.span),
                };

                match tokens.next().map(|t| t.variant) {
                    Some(Reserved(Requirement)) => {}
                    _ => return Err(token.span),
                }

                let node = Node::new(meta, Variant::Requirement(req));
                let idx = draft.push_node(node, token.span);
                return Ok(idx);
            }

            _ => return Err(tok.span),
        }
    }

    Err(Span::default())
}

fn is_closing(tokens: &mut MultiPeek<TokenIterator>, stop: &Keyword) -> bool {
    use TokenVariant::*;

    let ret = match tokens.peek().map(|t| &t.variant) {
        Some(Reserved(keyword)) if keyword == stop => match tokens.peek().map(|t| &t.variant) {
            Some(Identifier(_)) => false,
            _ => true,
        },
        _ => false,
    };

    tokens.reset_peek();
    ret
}

fn parse_type(expr: &str) -> Result<Vec<String>, Span> {
    Ok(expr.split('|').map(|m| m.trim().to_string()).collect())
}

fn parse_children(
    tokens: &mut MultiPeek<TokenIterator>,
    stop: &Keyword,
    draft: &mut Draft,
) -> Result<Vec<Idx>, Span> {
    let mut children = Vec::new();

    while !is_closing(tokens, stop) {
        children.push(parse_tokens(tokens, draft)?);
    }
    tokens.next();

    Ok(children)
}

fn parse_identifier(tokens: &mut MultiPeek<TokenIterator>) -> Result<String, Span> {
    use TokenVariant::*;

    match tokens.next() {
        Some(tok) => match tok.variant {
            Identifier(identifier) => Ok(identifier.into()),
            _ => Err(tok.span),
        },
        None => Ok(String::default()),
    }
}

fn parse_label(tokens: &mut MultiPeek<TokenIterator>) -> Result<String, Span> {
    use TokenVariant::*;

    match tokens.next() {
        Some(tok) => match tok.variant {
            Identifier(label) if label.contains("::") => Err(tok.span),
            Identifier(label) => Ok(label.to_string()),
            _ => Err(tok.span),
        },
        None => Ok(String::new()),
    }
}

fn is_import(tokens: &mut MultiPeek<TokenIterator>) -> bool {
    use Keyword::*;
    use TokenVariant::*;

    let ret = match tokens.peek().map(|t| &t.variant) {
        Some(Reserved(Import)) => match tokens.peek().map(|t| &t.variant) {
            Some(Identifier(_)) => true,
            _ => false,
        },
        _ => false,
    };

    tokens.reset_peek();
    ret
}

fn parse_imports(
    tokens: &mut MultiPeek<TokenIterator>,
    draft: &mut Draft<'_>,
) -> Result<Vec<reference::Idx>, Span> {
    use TokenVariant::*;

    let mut imports = Vec::new();
    while is_import(tokens) {
        let _ = tokens.next().unwrap();
        let id = tokens.next().unwrap();
        match id.variant {
            Identifier(ident) => imports.push(draft.push_reference(ident.into())),
            _ => unreachable!(),
        }
    }

    return Ok(imports);
}

impl TraceabilityKind {
    fn is_traceability_keyword(value: &Keyword) -> bool {
        match value {
            Keyword::Refines => true,
            Keyword::Specializes => true,
            Keyword::Derives => true,
            _ => false,
        }
    }
}

impl<'a> TryFrom<&'a Keyword> for TraceabilityKind {
    type Error = &'a Keyword;

    fn try_from(value: &'a Keyword) -> Result<Self, Self::Error> {
        match value {
            Keyword::Refines => Ok(TraceabilityKind::Refinement),
            Keyword::Specializes => Ok(TraceabilityKind::Specialization),
            Keyword::Derives => Ok(TraceabilityKind::Derivation),
            v => Err(v),
        }
    }
}
