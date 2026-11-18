use crate::{
    node::{
        self,
        entity::{
            self, Attribute, Idx, IdxMarker, Meta, Node, RequirementVariant, Tag, TraceabilityKind,
            TraceabilityLink, Variant,
        },
        reference,
    },
    syntax::{
        location::Span,
        parser::Parser,
        tokenizer::entity::{Keyword, Token, TokenVariant},
    },
};

pub type TokenArena = super::TokenArena<Token, IdxMarker>;
pub type ParserArena = super::ParserArena<Token, Node, IdxMarker>;

impl TokenArena {
    pub fn name_of(&self, idx: entity::Idx) -> Option<Span> {
        use TokenVariant::*;

        self.tokens_of(idx)
            .iter()
            .find(|t| t.variant == Identifier)
            .map(|t| t.span)
    }
}

impl Parser<'_> {
    /// `stop` is the keyword that closes the entity this one is in, if any
    pub fn entity(&mut self, stop: Option<&Keyword>) -> Option<Idx> {
        let meta = self.parse_meta();
        self.skip_to_next_entity(stop);
        if stop.is_some_and(|stop| self.is_closing(stop)) {
            return None;
        }
        self.parse_entity(meta)
    }

    pub fn skip_to_next_entity(&mut self, stop: Option<&Keyword>) {
        use Keyword::*;

        while let Some(token) = self.entities.tokens.peek() {
            if stop.is_some_and(|stop| self.is_closing(stop)) {
                return;
            }
            if let TokenVariant::Reserved(Part | Package | Requirement | Let | Type) = token.variant
            {
                if let Some(tok) = self.entities.tokens.peek_nth(1)
                    && let TokenVariant::Identifier = tok.variant
                {
                    return;
                }
                self.errors.push(token.span);
            }
            self.entities.tokens.bump();
        }
    }

    fn parse_meta(&mut self) -> Meta {
        use TokenVariant::*;

        let mut meta = Meta::default();

        while let Some(tok) = self.entities.tokens.peek() {
            match tok.variant {
                Markup => {
                    meta.comment = Some(self.markup(tok.span));
                    self.entities.tokens.bump();
                }

                Tag => {
                    meta.tags.push(self.parse_tag(&tok));
                    self.entities.tokens.bump();
                }

                _ => break,
            }
        }

        meta
    }

    fn parse_tag(&self, token: &Token) -> Tag {
        let text = token.text(self.source);
        let (id, value) = match text.split_once(char::is_whitespace) {
            Some((id, value)) => (id, Some(value.trim_start())),
            None => (text, None),
        };
        entity::Tag::new(id.to_string(), value.map(String::from))
    }

    fn parse_entity(&mut self, mut meta: Meta) -> Option<Idx> {
        use Keyword::*;
        use TokenVariant::*;

        let source = self.source;
        let start = self.entities.tokens.position();
        let tok = self.entities.tokens.bump()?;
        match tok.variant {
            Reserved(Part) => {
                let mut prt = node::entity::Part::default();

                meta.label = self.parse_label()?;
                prt.imports = self.parse_imports();
                prt.children = self.parse_children(&Part);
                let node = Node::new(meta, Variant::Part(prt));
                Some(self.entities.push_from(node, start))
            }

            Reserved(Package) => {
                let mut pkg = node::entity::Package::default();

                meta.label = self.parse_label()?;
                pkg.imports = self.parse_imports();
                pkg.children = self.parse_children(&Package);
                let node = Node::new(meta, Variant::Package(pkg));
                Some(self.entities.push_from(node, start))
            }

            Reserved(Let) => {
                let label = self.parse_label()?;
                self.expect(Reserved(In), tok.span)?;
                let token = self.expect(Expression, tok.span)?;
                let domain = self.expression(token.span)?;

                let mut atr = Attribute::new(domain);

                meta.label = label;
                atr.unit = match self.entities.tokens.peek() {
                    Some(token) if token.variant == TokenVariant::Unit => {
                        self.entities.tokens.bump();
                        Some(token.text(source).to_string())
                    }

                    _ => None,
                };

                let node = Node::new(meta, Variant::Attribute(atr));
                Some(self.entities.push_from(node, start))
            }

            Reserved(Keyword::Type) => {
                let mut typ = node::entity::Type::default();
                meta.label = self.parse_label()?;
                self.expect(Reserved(Has), tok.span)?;
                let members = self.expect(TokenVariant::Type, tok.span)?;
                typ.members = parse_type(members.text(source));
                self.expect(Reserved(Keyword::Type), tok.span)?;

                let node = Node::new(meta, Variant::Type(typ));
                Some(self.entities.push_from(node, start))
            }

            Reserved(Requirement) => {
                // The variant is only known after the header
                let mut restriction = None;
                let mut allocation = None;
                let mut traceability = Vec::new();
                let mut inhibitor = node::entity::TraceabilityInhibitor::default();
                meta.label = self.parse_label()?;
                while let Some(token) = self.entities.tokens.bump() {
                    match token.variant {
                        Reserved(Keyword::Is) => {
                            break;
                        }

                        Reserved(Keyword::Top) => inhibitor.top = true,

                        Reserved(Keyword::Bottom) => inhibitor.bottom = true,

                        Reserved(Keyword::On) if let Some((_, span)) = self.parse_ident() => {
                            restriction = Some(self.reference(span));
                        }

                        Reserved(Keyword::Via) if let Some((_, span)) = self.parse_ident() => {
                            allocation = Some(self.reference(span));
                        }

                        Reserved(ref k)
                            if let Ok(kind) = TraceabilityKind::try_from(k)
                                && let Some((_, span)) = self.parse_ident() =>
                        {
                            let link = TraceabilityLink::new(kind, self.reference(span));
                            traceability.push(link);
                        }

                        _ => {
                            self.errors.push(token.span);
                            return None;
                        }
                    };
                }

                let token = self.entities.tokens.bump()?;

                use RequirementVariant::*;
                let variant = match token.variant {
                    Markup => Informal(self.markup(token.span)),
                    Expression => match self.expression(token.span) {
                        Some(expression) => Formal(expression),
                        None => {
                            self.entities.tokens.bump();
                            return None;
                        }
                    },
                    _ => {
                        self.errors.push(token.span);
                        return None;
                    }
                };

                self.entities.tokens.bump()?;

                let req = node::entity::Requirement {
                    restriction,
                    allocation,
                    traceability,
                    inhibitor,
                    variant,
                };
                let node = Node::new(meta, Variant::Requirement(req));
                Some(self.entities.push_from(node, start))
            }

            _ => None,
        }
    }

    fn is_closing(&self, stop: &Keyword) -> bool {
        use TokenVariant::*;

        match self.entities.tokens.peek().map(|t| t.variant) {
            Some(Reserved(keyword)) if keyword == *stop => match self.entities.tokens.peek_nth(1) {
                Some(token) => token.variant != Identifier,
                None => true,
            },
            _ => false,
        }
    }

    fn parse_children(&mut self, stop: &Keyword) -> Vec<Idx> {
        let mut children = Vec::new();

        while self.entities.tokens.peek().is_some() && !self.is_closing(stop) {
            if let Some(node) = self.entity(Some(stop)) {
                children.push(node);
            }
        }
        self.entities.tokens.bump();

        children
    }

    fn parse_ident(&mut self) -> Option<(String, Span)> {
        let source = self.source;
        let tok = self.entities.tokens.bump()?;
        match tok.variant {
            TokenVariant::Identifier => Some((tok.text(source).to_string(), tok.span)),
            _ => {
                self.errors.push(tok.span);
                None
            }
        }
    }

    /// Consumes the next token if it is a `variant`, else records it as an error and leaves it
    /// to what follows. At the end of the input, the error is on `entity`, the span of the keyword
    /// of the entity that expects it.
    fn expect(&mut self, variant: TokenVariant, entity: Span) -> Option<Token> {
        self.entities
            .tokens
            .expect(variant)
            .map_err(|span| self.errors.push(span.unwrap_or(entity)))
            .ok()
    }

    fn parse_label(&mut self) -> Option<String> {
        let source = self.source;
        let tok = self.entities.tokens.bump()?;
        match tok.variant {
            TokenVariant::Identifier => {
                let label = tok.text(source);
                if label.contains("::") {
                    self.errors.push(tok.span);
                }
                Some(label.to_string())
            }
            _ => {
                self.errors.push(tok.span);
                None
            }
        }
    }

    fn is_import(&self) -> bool {
        use Keyword::*;
        use TokenVariant::*;

        match self.entities.tokens.peek().map(|t| t.variant) {
            Some(Reserved(Import)) => {
                matches!(self.entities.tokens.peek_nth(1), Some(t) if t.variant == Identifier)
            }
            _ => false,
        }
    }

    fn parse_imports(&mut self) -> Vec<reference::Idx> {
        let mut imports = Vec::new();
        while self.is_import() {
            self.entities.tokens.bump();
            let id = self.entities.tokens.bump().unwrap();
            imports.push(self.reference(id.span));
        }

        imports
    }
}

fn parse_type(expr: &str) -> Vec<String> {
    expr.split('|').map(|m| m.trim().to_string()).collect()
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
