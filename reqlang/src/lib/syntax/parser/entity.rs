use itertools::MultiPeek;

use crate::{
    node::{
        self,
        entity::{
            self, Attribute, Idx, Meta, RequirementVariant, TraceabilityKind, TraceabilityLink,
            Variant,
        },
    },
    reference::Reference,
    syntax::{
        location::Span,
        parser::{expression, markup},
        tokenizer::entity::{Keyword, TokenIterator, TokenVariant},
    },
};

pub type Node = node::entity::Node<Span, Reference, Reference, Span, ()>;
// INVARIANT: Always parsed in DFS order (to allow iterating directly over the Vec<Node>)
// TODO: Use peek_nth to avoid reset_peek
pub fn parse(tokens: &mut MultiPeek<TokenIterator>, arena: &mut Vec<Node>) -> Result<Idx, Span> {
    use Keyword::*;
    use TokenVariant::*;

    let mut meta = Meta::default();

    while let Some(tok) = tokens.next() {
        match tok.variant {
            Markup(comment) => {
                let markup = markup::Node::from(comment.to_string());
                meta.comment = Some(markup);
            }

            Tag(id, value) => {
                let tag = entity::Tag::new(id.to_string(), value.map(String::from));
                meta.tags.push(tag);
            }

            Reserved(Part) => {
                let mut prt = node::entity::Part::default();

                meta.label = parse_label(tokens)?;
                prt.imports = parse_imports(tokens)?;
                let idx = arena.len();
                arena.push(Node::new(tok.span, meta, Variant::Part(prt)));
                let children = parse_children(tokens, &Part, arena)?;
                arena[idx].variant.children_mut().unwrap().extend(children);
                return Ok(idx.into());
            }

            Reserved(Package) => {
                let mut pkg = node::entity::Package::default();

                meta.label = parse_label(tokens)?;
                pkg.imports = parse_imports(tokens)?;
                let idx = arena.len();
                arena.push(Node::new(tok.span, meta, Variant::Package(pkg)));
                let children = parse_children(tokens, &Package, arena)?;
                arena[idx].variant.children_mut().unwrap().extend(children);
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
                                expression::parse(expr, &token.span.start)?
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

                let idx = arena.len();
                arena.push(Node::new(span.clone(), meta, Variant::Attribute(atr)));
                return Ok(idx.into());
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
                                Ok(ident) => Some(Reference::new(ident)),
                                Err(span) => return Err(span),
                            };
                        }

                        Reserved(Keyword::Via) => {
                            req.allocation = match parse_identifier(tokens) {
                                Ok(ident) => Some(Reference::new(ident)),
                                Err(span) => return Err(span),
                            };
                        }

                        Reserved(ref k) if TraceabilityKind::is_traceability_keyword(&k) => {
                            let kind = TraceabilityKind::try_from(k).unwrap();
                            let target = match parse_identifier(tokens) {
                                Ok(ident) => Reference::new(ident),
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
                    Markup(expr) => Informal(markup::Node::from(expr.to_string())),
                    Expression(expr) => Formal(expression::parse(expr, &token.span.start)?),
                    _ => return Err(token.span),
                };

                match tokens.next().map(|t| t.variant) {
                    Some(Reserved(Requirement)) => {}
                    _ => return Err(token.span),
                }

                let idx = arena.len();
                arena.push(Node::new(token.span, meta, Variant::Requirement(req)));
                return Ok(idx.into());
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

fn parse_children(
    tokens: &mut MultiPeek<TokenIterator>,
    stop: &Keyword,
    arena: &mut Vec<Node>,
) -> Result<Vec<Idx>, Span> {
    let mut children = Vec::new();

    while !is_closing(tokens, stop) {
        children.push(parse(tokens, arena)?);
    }
    tokens.next();

    Ok(children)
}

fn parse_identifier(tokens: &mut MultiPeek<TokenIterator>) -> Result<String, Span> {
    use TokenVariant::*;

    match tokens.next() {
        Some(tok) => match tok.variant {
            Identifier(identifier) => Ok(identifier.to_string()),
            _ => Err(tok.span),
        },
        None => Ok(String::new()),
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

fn parse_imports(tokens: &mut MultiPeek<TokenIterator>) -> Result<Vec<Reference>, Span> {
    use TokenVariant::*;

    let mut imports = Vec::new();
    while is_import(tokens) {
        let _ = tokens.next().unwrap();
        let id = tokens.next().unwrap();
        match id.variant {
            Identifier(ident) => imports.push(Reference::new(ident.to_string())),
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
