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
        tokenizer::entity::{Keyword, Token, TokenVariant},
    },
    utils::cursor::Cursor,
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

pub(super) fn parse_tree(
    tokens: &mut Cursor<Token>,
    draft: &mut Draft,
    errors: &mut Vec<Span>,
) -> Option<Idx> {
    let meta = parse_meta(tokens, draft);
    skip_to_next_entity(tokens);
    parse_entity(meta, tokens, draft, errors)
}

pub fn skip_to_next_entity(tokens: &mut Cursor<Token>) {
    use Keyword::*;

    while let Some(token) = tokens.peek() {
        if let TokenVariant::Reserved(k) = &token.variant {
            match k {
                Part | Package | Requirement | Let | Type => return,
                _ => {
                    tokens.bump();
                }
            }
        }
    }
}

fn parse_meta(tokens: &mut Cursor<Token>, draft: &mut Draft) -> Meta {
    use TokenVariant::*;

    let mut meta = Meta::default();

    while let Some(tok) = tokens.peek() {
        match tok.variant {
            Markup(comment) => {
                let markup = markup::parse(&comment.to_string(), &mut draft.tree.references);
                meta.comment = Some(markup);
                tokens.bump();
            }

            Tag(id, value) => {
                let tag = entity::Tag::new(id.to_string(), value.map(String::from));
                meta.tags.push(tag);
                tokens.bump();
            }

            _ => break,
        }
    }

    meta
}

fn parse_entity(
    mut meta: Meta,
    tokens: &mut Cursor<Token>,
    draft: &mut Draft,
    errors: &mut Vec<Span>,
) -> Option<Idx> {
    use Keyword::*;
    use TokenVariant::*;

    let tok = tokens.bump()?;
    match tok.variant {
        Reserved(Part) => {
            let mut prt = node::entity::Part::default();

            meta.label = parse_label(tokens, errors)?;
            prt.imports = parse_imports(tokens, draft);
            prt.children = parse_children(tokens, &Part, draft, errors);
            let node = Node::new(meta, Variant::Part(prt));
            let idx = draft.push_node(node, tok.span);
            return Some(idx.into());
        }

        Reserved(Package) => {
            let mut pkg = node::entity::Package::default();

            meta.label = parse_label(tokens, errors)?;
            pkg.imports = parse_imports(tokens, draft);
            pkg.children = parse_children(tokens, &Package, draft, errors);
            let node = Node::new(meta, Variant::Package(pkg));
            let idx = draft.push_node(node, tok.span);
            return Some(idx.into());
        }

        Reserved(Let) => {
            let label = parse_label(tokens, errors)?;
            let token = tokens.bump()?;
            let domain = match token.variant {
                TokenVariant::Reserved(Keyword::In) => {
                    let token = tokens.bump()?;

                    match token.variant {
                        TokenVariant::Expression(expr) => {
                            let draft = expression::Draft::from(&mut *draft);
                            expression::parse(expr, &token.span.start, draft).ok()?
                        }

                        _ => return None,
                    }
                }
                _ => return None,
            };

            let mut atr = Attribute::new(domain);

            meta.label = label;
            let token = tokens.peek()?;
            let span = token.span.clone();
            atr.unit = match token.variant {
                TokenVariant::Unit(expr) => {
                    tokens.bump();
                    Some(expr.to_string())
                }

                _ => None,
            };

            let node = Node::new(meta, Variant::Attribute(atr));
            let idx = draft.push_node(node, span);

            return Some(idx);
        }

        Reserved(Keyword::Type) => {
            use TokenVariant::*;
            let mut typ = node::entity::Type::default();
            meta.label = parse_label(tokens, errors)?;
            match tokens.bump() {
                None => return None,
                Some(tok) if let Reserved(Has) = tok.variant => {}
                _ => return None,
            }

            typ.members = match tokens.bump() {
                None => return None,
                Some(tok) if let Type(expr) = tok.variant => parse_type(expr),
                _ => return None,
            };

            tokens.bump()?;

            let node = Node::new(meta, Variant::Type(typ));
            let idx = draft.push_node(node, tok.span);
            return Some(idx);
        }

        Reserved(Requirement) => {
            use TokenVariant::*;
            let mut req = node::entity::Requirement::default();
            meta.label = parse_label(tokens, errors)?;
            while let Some(token) = tokens.bump() {
                match token.variant {
                    Reserved(Keyword::Is) => {
                        break;
                    }

                    Reserved(Keyword::Top) => req.inhibitor.top = true,

                    Reserved(Keyword::Bottom) => req.inhibitor.bottom = true,

                    Reserved(Keyword::On) if let Some(ident) = parse_ident(tokens, errors) => {
                        req.restriction = Some(draft.push_reference(ident.into()));
                    }

                    Reserved(Keyword::Via) if let Some(ident) = parse_ident(tokens, errors) => {
                        req.allocation = Some(draft.push_reference(ident.into()));
                    }

                    Reserved(ref k)
                        if let Ok(kind) = TraceabilityKind::try_from(k)
                            && let Some(ident) = parse_ident(tokens, errors) =>
                    {
                        let link = TraceabilityLink::new(kind, draft.push_reference(ident.into()));
                        req.traceability.push(link);
                    }

                    _ => return None,
                };
            }

            let token = tokens.bump()?;

            use RequirementVariant::*;
            req.variant = match token.variant {
                Markup(expr) => {
                    Informal(markup::parse(&expr.to_string(), &mut draft.tree.references))
                }
                Expression(expr) => {
                    let draft = expression::Draft::from(&mut *draft);
                    Formal(expression::parse(expr, &token.span.start, draft).ok()?)
                }
                _ => return None,
            };

            tokens.bump()?;

            let node = Node::new(meta, Variant::Requirement(req));
            let idx = draft.push_node(node, token.span);
            return Some(idx);
        }

        _ => return None,
    };
}

fn is_closing(tokens: &mut Cursor<Token>, stop: &Keyword) -> bool {
    use TokenVariant::*;

    let ret = match tokens.peek().map(|t| &t.variant) {
        Some(Reserved(keyword)) if keyword == stop => {
            match tokens.peek_nth(1).map(|t| &t.variant) {
                Some(Identifier(_)) => false,
                _ => true,
            }
        }
        _ => false,
    };
    ret
}

fn parse_type(expr: &str) -> Vec<String> {
    expr.split('|').map(|m| m.trim().to_string()).collect()
}

fn parse_children(
    tokens: &mut Cursor<Token>,
    stop: &Keyword,
    draft: &mut Draft,
    errors: &mut Vec<Span>,
) -> Vec<Idx> {
    let mut children = Vec::new();

    while !is_closing(tokens, stop) {
        if let Some(node) = parse_tree(tokens, draft, errors) {
            children.push(node);
        }
    }
    tokens.bump();

    children
}

fn parse_ident(tokens: &mut Cursor<Token>, errors: &mut Vec<Span>) -> Option<String> {
    use TokenVariant::*;

    match tokens.bump() {
        Some(tok) => match tok.variant {
            Identifier(identifier) => Some(identifier.into()),
            _ => {
                errors.push(tok.span);
                None
            }
        },
        None => None,
    }
}

fn parse_label(tokens: &mut Cursor<Token>, errors: &mut Vec<Span>) -> Option<String> {
    use TokenVariant::*;

    match tokens.bump() {
        Some(tok) => match tok.variant {
            Identifier(label) if label.contains("::") => {
                errors.push(tok.span);
                Some(label.to_string())
            }
            Identifier(label) => Some(label.to_string()),
            _ => {
                errors.push(tok.span);
                None
            }
        },
        None => None,
    }
}

fn is_import(tokens: &mut Cursor<Token>) -> bool {
    use Keyword::*;
    use TokenVariant::*;

    let ret = match tokens.peek().map(|t| &t.variant) {
        Some(Reserved(Import)) => match tokens.peek_nth(1).map(|t| &t.variant) {
            Some(Identifier(_)) => true,
            _ => false,
        },
        _ => false,
    };

    ret
}

fn parse_imports(tokens: &mut Cursor<Token>, draft: &mut Draft<'_>) -> Vec<reference::Idx> {
    use TokenVariant::*;

    let mut imports = Vec::new();
    while is_import(tokens) {
        let _ = tokens.bump().unwrap();
        let id = tokens.bump().unwrap();
        match id.variant {
            Identifier(ident) => imports.push(draft.push_reference(ident.into())),
            _ => unreachable!(),
        }
    }

    return imports;
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
