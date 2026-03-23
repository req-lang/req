use crate::{
    children::ChildrenMut,
    syntax::{
        entity::{
            self, Attribute, Entity, EntityMeta, EntityVariant, Package, Part, Reference,
            Requirement, TraceabilityInhibitor, TraceabilityKind, TraceabilityLink,
            tokenizer::{Keyword, TokenIterator, TokenVariant, try_consume_identifier},
        },
        error::Error,
    },
};
use crate::{
    syntax::{NodeParser, Span, entity::RequirementVariant, error::ErrorKind},
    visitor::{Visitor, Walk, WalkCustom},
};
use std::{iter::Peekable, ops::ControlFlow};

use super::tokenizer::{IntoLexeme, IntoPosition, IntoTokens};

impl<'a> NodeParser<'a> {
    pub fn parse(&mut self, input: &'a str) -> Result<Entity, Error> {
        let tokens = input.char_indices().positioned().lexemes().tokens();
        match self.walk(&mut tokens.peekable()) {
            ControlFlow::Continue(node) => Ok(node),
            ControlFlow::Break(error) => Err(error),
        }
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub enum State {
    #[default]
    Specification,
    Package,
    Part,
    Attribute,
    Requirement,
}

impl TryFrom<Keyword> for State {
    type Error = Keyword;

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        match value {
            Keyword::Package => Ok(State::Package),
            Keyword::Part => Ok(State::Part),
            Keyword::Requirement => Ok(State::Requirement),
            Keyword::Let => Ok(State::Attribute),
            v => Err(v),
        }
    }
}

impl PartialEq<Keyword> for State {
    fn eq(&self, other: &Keyword) -> bool {
        match (self, other) {
            (State::Package, Keyword::Package) => true,
            (State::Part, Keyword::Part) => true,
            (State::Requirement, Keyword::Requirement) => true,
            (_, _) => false,
        }
    }
}

pub trait ContinueOrElse<B, C, F> {
    fn continue_or_else(self, err: F) -> ControlFlow<B, C>;
}

impl<B, C, F: FnOnce() -> B> ContinueOrElse<B, C, F> for Option<C> {
    fn continue_or_else(self, err: F) -> ControlFlow<B, C> {
        match self {
            Some(c) => ControlFlow::Continue(c),
            None => ControlFlow::Break(err()),
        }
    }
}

impl<B, C, E, F: FnOnce(E) -> B> ContinueOrElse<B, C, F> for Result<C, E> {
    fn continue_or_else(self, err: F) -> ControlFlow<B, C> {
        match self {
            Ok(c) => ControlFlow::Continue(c),
            Err(e) => ControlFlow::Break(err(e)),
        }
    }
}

impl<'a> Walk<&mut Peekable<TokenIterator<'a>>, Error, Entity> for NodeParser<'a> {
    fn walk(&mut self, tokens: &mut Peekable<TokenIterator<'a>>) -> ControlFlow<Error, Entity> {
        use ErrorKind::*;
        use Keyword::*;
        use TokenVariant::*;

        if self.state == State::Specification {
            let mut token = tokens.next().continue_or_else(|| Error::empty())?;
            loop {
                match token.variant {
                    Markup(comment) => {
                        self.offset = token.span.start;
                        let container = self.parse_markup(comment.to_string(), token.span.end);
                        self.last_comment = Some(container);
                    }

                    Tag(id, value) => {
                        self.last_tags.push((id, value));
                    }

                    _ => {
                        break;
                    }
                }
                token = tokens.next().continue_or_else(|| Error::empty())?;
            }
            self.start = token.span.start;
            self.state = State::Package;
            return self.walk(tokens);
        }

        let mut node = self.visit(tokens)?;

        if self.state == State::Attribute {
            return ControlFlow::Continue(node);
        }

        if self.state == State::Requirement {
            let token = tokens.next().continue_or_else(|| Error::empty())?;
            match token.variant {
                Reserved(keyword) if self.state == keyword => {
                    let span = Span::new(self.start, token.span.end);
                    self.spans.push(span);
                    return ControlFlow::Continue(node);
                }

                _ => {
                    let error = Error::from_token(&token, MissingEnd);
                    return ControlFlow::Break(error);
                }
            }
        }

        while let Some(token) = tokens.next() {
            if let Reserved(keyword) = &token.variant {
                if self.state == *keyword {
                    match tokens.peek() {
                        None => {
                            let span = Span::new(self.start, token.span.end);
                            self.spans.push(span);
                            break;
                        }

                        Some(t) => match t.variant {
                            Reserved(_) | Markup(_) | Tag(_, _) => {
                                let span = Span::new(self.start, token.span.end);
                                self.spans.push(span);
                                break;
                            }

                            Identifier(_) => {}

                            _ => {
                                let error = Error::from_token(&token, MissingEnd);
                                return ControlFlow::Break(error);
                            }
                        },
                    }
                }
            }

            match token.variant {
                Reserved(Import) => match try_consume_identifier(tokens) {
                    Ok((ident, _)) => {
                        let imports = node.imports_mut().unwrap();
                        imports.push(Reference::new(ident.to_string()));
                    }

                    _ => {
                        let error = Error::from_token(&token, MissingIdentifier);
                        self.errors.push(error);
                    }
                },

                Reserved(ref keyword) => match State::try_from(keyword.clone()) {
                    Ok(state) => {
                        let children = node.children_mut().unwrap();
                        let prev_start = self.start;
                        let prev_state = self.state.clone();

                        self.start = token.span.start;
                        self.state = state;
                        children.push(self.walk(&mut *tokens)?);
                        self.start = prev_start;
                        self.state = prev_state;
                    }

                    Err(_) => {
                        let error = Error::from_token(&token, UnexpectedKeyword);
                        self.errors.push(error);
                    }
                },

                Expression(_) | Unit(_) | Identifier(_) => {
                    let error = Error::from_token(&token, UnexpectedExpression);
                    self.errors.push(error);
                }

                Markup(comment) => {
                    self.offset = token.span.start;
                    let container = self.parse_markup(comment.to_string(), token.span.end);
                    self.last_comment = Some(container);
                }

                Tag(id, value) => {
                    let id = id;
                    self.last_tags.push((id, value));
                }
            }
        }
        ControlFlow::Continue(node)
    }
}

impl Visitor<&mut Peekable<TokenIterator<'_>>, Error, Entity> for NodeParser<'_> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, tokens: &mut Peekable<TokenIterator<'_>>) -> ControlFlow<Error, Entity> {
        use ControlFlow::*;
        use ErrorKind::*;
        use TokenVariant::*;

        let mut entity = EntityMeta::default();
        let tags = std::mem::take(&mut self.last_tags);
        entity.tags = tags
            .into_iter()
            .map(|(id, val)| entity::Tag::new(id.into(), val.map(|v| v.into())))
            .collect();
        entity.comment = std::mem::take(&mut self.last_comment);

        match try_consume_identifier(tokens) {
            Ok((l, span)) if l.contains("::") => Break(Error::from_span(span, BadIdentifier))?,
            Ok(l) => entity.label = l.0.to_string(),
            Err(Some(tok)) => Break(Error::from_token(&tok, MissingIdentifier))?,
            Err(None) => Break(Error::empty())?,
        };

        let variant = match self.state {
            State::Package => {
                let package = Package::default();
                EntityVariant::Package(package)
            }

            State::Part => {
                let package = Part::default();
                EntityVariant::Part(package)
            }

            State::Attribute => {
                let token = tokens.next().continue_or_else(|| Error::empty())?;
                let mut end;
                let domain = match token.variant {
                    TokenVariant::Reserved(Keyword::In) => {
                        let token = tokens
                            .next()
                            .continue_or_else(|| Error::unterminated(&token))?;

                        match token.variant {
                            TokenVariant::Expression(expr) => {
                                end = token.span.end;
                                self.offset = token.span.start;
                                self.parse_expr(expr)?
                            }

                            _ => {
                                let error = Error::from_token(&token, IllFormedAttribute);
                                return ControlFlow::Break(error);
                            }
                        }
                    }

                    _ => {
                        let error = Error::from_token(&token, IllFormedAttribute);
                        return ControlFlow::Break(error);
                    }
                };
                let mut attribute = Attribute::new(domain);
                let token = tokens
                    .peek()
                    .continue_or_else(|| Error::unterminated(&token))?;

                attribute.unit = match token.variant {
                    TokenVariant::Unit(expr) => {
                        end = token.span.end;
                        self.offset = token.span.start;
                        tokens.next();

                        Some(self.parse_expr(expr)?)
                    }

                    _ => None,
                };
                let span = Span::new(self.start, end);
                self.spans.push(span);
                EntityVariant::Attribute(attribute)
            }

            State::Requirement => {
                let mut requirement = Requirement::new();
                while let Some(token) = tokens.next() {
                    use ControlFlow::*;
                    use TraceabilityInhibitor::*;

                    match token.variant {
                        Reserved(Keyword::Is) => {
                            break;
                        }

                        Reserved(Keyword::On) => {
                            requirement.restriction = match try_consume_identifier(tokens) {
                                Ok((r, _)) => Some(Reference::new(r.into())),
                                Err(Some(tok)) => Break(Error::missing_id(&tok))?,
                                Err(None) => Break(Error::unterminated(&token))?,
                            };
                        }

                        Reserved(Keyword::Via) => {
                            requirement.allocation = match try_consume_identifier(tokens) {
                                Ok((r, _)) => Some(Reference::new(r.into())),
                                Err(Some(tok)) => Break(Error::missing_id(&tok))?,
                                Err(None) => Break(Error::unterminated(&token))?,
                            };
                        }

                        Reserved(Keyword::Top) => match requirement.inhibitor {
                            None => requirement.inhibitor = Some(Top),
                            Some(Bottom) => requirement.inhibitor = Some(Isolated),
                            _ => {}
                        },

                        Reserved(Keyword::Bottom) => match requirement.inhibitor {
                            None => requirement.inhibitor = Some(Bottom),
                            Some(Top) => requirement.inhibitor = Some(Isolated),
                            _ => {}
                        },

                        Reserved(ref k) if TraceabilityKind::is_traceability_keyword(&k) => {
                            let kind = TraceabilityKind::try_from(k).unwrap();
                            let target = match try_consume_identifier(tokens) {
                                Ok((r, _)) => Reference::new(r.into()),
                                Err(Some(tok)) => Break(Error::missing_id(&tok))?,
                                Err(None) => Break(Error::unterminated(&token))?,
                            };
                            let link = TraceabilityLink::new(kind, target);
                            requirement.traceability.push(link);
                        }

                        _ => {
                            let error = Error::from_token(&token, IllFormedRequirement);
                            Break(error)?;
                        }
                    };
                }
                use RequirementVariant::*;

                let token = tokens.next().continue_or_else(|| Error::empty())?;

                self.offset = token.span.start;
                let end = token.span.end;
                let variant = match token.variant {
                    Markup(expr) => Informal(self.parse_markup(expr.to_string(), end)),
                    Expression(expr) => Formal(self.parse_expr(expr)?),
                    _ => {
                        let error = Error::from_token(&token, MissingExpression);
                        return ControlFlow::Break(error);
                    }
                };
                requirement.variant = variant;

                EntityVariant::Requirement(requirement)
            }

            _ => unreachable!(),
        };

        ControlFlow::Continue(Entity::new(entity, variant))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        children::{Children, ChildrenMut},
        mock::generator::{self, Generate},
        renderer,
        syntax::{
            entity::{Attribute, Entity, EntityVariant, Part, Reference, TraceabilityKind},
            expression,
        },
        visitor::Walk,
    };

    use super::NodeParser;

    #[test]
    fn parses_simple_input() {
        let input = "package x package";
        let mut parser = NodeParser::default();
        let result = parser.parse(input);

        let mut expected = Entity::default();
        expected.meta.label = "x".to_string();

        assert_eq!(result.is_ok(), true);
        assert_eq!(parser.errors.len(), 0);
        assert_eq!(result.ok().unwrap(), expected);
    }

    #[test]
    fn parses_input_with_part_and_attribute() {
        let input = "package x part p let u in real [m] let v in integer part package";
        let mut parser = NodeParser::default();
        let result = parser.parse(input);

        let mut expected = Entity::default();
        expected.meta.label = "x".to_string();

        let mut expr = expression::Expression::default();
        let reference = Reference::new("real".to_string());
        let id = expression::Identifier::new(reference);
        expr.variant = expression::ExpressionVariant::Identifier(id);

        let mut unit = expression::Expression::default();
        let reference = Reference::new("m".to_string());
        let id = expression::Identifier::new(reference);
        unit.variant = expression::ExpressionVariant::Identifier(id);

        let mut second = Entity::default();
        let mut attribute = Attribute::new(expr);
        attribute.unit = Some(unit);
        second.variant = EntityVariant::Attribute(attribute);
        second.meta.label = "u".to_string();

        let mut expr = expression::Expression::default();
        let reference = Reference::new("integer".to_string());
        let id = expression::Identifier::new(reference);
        expr.variant = expression::ExpressionVariant::Identifier(id);

        let mut third = Entity::default();
        let attribute = Attribute::new(expr);
        third.variant = EntityVariant::Attribute(attribute);
        third.meta.label = "v".to_string();

        let mut first = Entity::default();
        let part = Part::new(Vec::new(), vec![second, third]);
        first.variant = EntityVariant::Part(part);
        first.meta.label = "p".to_string();

        expected.children_mut().unwrap().push(first);

        if let Err(error) = &result {
            eprintln!("{}", error);
        }

        for error in &parser.errors {
            eprintln!("{}", error);
        }

        assert_eq!(result.is_ok(), true);
        assert_eq!(parser.errors.len(), 0);

        let actual = result.ok().unwrap();
        let mut renderer = renderer::formatter::Renderer::new();
        let _ = renderer.walk(&actual);
        let output = renderer.source;

        println!("*** Actual ***");
        for (idx, line) in output.lines().enumerate() {
            println!("{:3} {}", idx, line);
        }

        assert_eq!(actual, expected);
    }

    #[test]
    fn parses_simple_generated_input() {
        let mut generator = generator::Simple::new();
        generator.packages = 2;
        generator.depth = 1;
        generator.words = 2;
        let expected = generator.generate();
        let mut renderer = renderer::formatter::Renderer::new();
        let _ = renderer.walk(&expected);
        let input = renderer.source;

        println!("*** Expected ***");
        for (idx, line) in input.lines().enumerate() {
            println!("{:3} {}", idx, line);
        }

        let mut parser = NodeParser::default();
        let result = parser.parse(&input);

        if let Err(error) = &result {
            eprintln!("{}", error);
        }

        for error in &parser.errors {
            eprintln!("{}", error);
        }

        assert_eq!(result.is_ok(), true);

        let actual = result.ok().unwrap();

        let mut renderer = renderer::formatter::Renderer::new();
        let _ = renderer.walk(&actual);
        let output = renderer.source;

        println!("*** Actual ***");
        for (idx, line) in output.lines().enumerate() {
            println!("{:3} {}", idx, line);
        }

        if input.lines().count() == output.lines().count() {}

        assert_eq!(parser.errors.len(), 0);
        assert_eq!(actual.iter().count(), generator.size() as usize);
        assert_eq!(actual, expected);
    }

    fn get_requirement(input: &str) -> crate::syntax::entity::Requirement {
        let mut parser = NodeParser::default();
        let result = parser.parse(input).expect("parse failed");
        assert_eq!(parser.errors.len(), 0);
        let entity = result
            .children()
            .unwrap()
            .iter()
            .find(|e| matches!(&e.variant, EntityVariant::Requirement(_)))
            .expect("no requirement found")
            .clone();
        match entity.variant {
            EntityVariant::Requirement(r) => r,
            _ => unreachable!(),
        }
    }

    #[test]
    fn parses_requirement_with_refinement_traceability() {
        let req =
            get_requirement("package p requirement x refines a is @@ text @@ requirement package");
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
    }

    #[test]
    fn parses_requirement_with_specialization_traceability() {
        let req = get_requirement(
            "package p requirement x specializes b is @@ text @@ requirement package",
        );
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[0].target.value, "b");
    }

    #[test]
    fn parses_requirement_with_derivation_traceability() {
        let req =
            get_requirement("package p requirement x derives c is @@ text @@ requirement package");
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Derivation);
        assert_eq!(req.traceability[0].target.value, "c");
    }

    #[test]
    fn parses_requirement_with_multiple_traceability_links() {
        let req = get_requirement(
            "package p requirement x refines a specializes b derives c is @@ text @@ requirement package",
        );
        assert_eq!(req.traceability.len(), 3);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.value, "b");
        assert_eq!(req.traceability[2].kind, TraceabilityKind::Derivation);
        assert_eq!(req.traceability[2].target.value, "c");
    }

    #[test]
    fn parses_requirement_with_constraint() {
        let req = get_requirement("package p requirement x on y is @@ text @@ requirement package");
        assert_eq!(
            req.restriction.as_ref().map(|r| r.value.as_str()),
            Some("y")
        );
        assert!(req.traceability.is_empty());
        assert!(req.allocation.is_none());
    }

    #[test]
    fn parses_requirement_with_allocation() {
        let req =
            get_requirement("package p requirement x via z is @@ text @@ requirement package");
        assert_eq!(req.allocation.as_ref().map(|r| r.value.as_str()), Some("z"));
        assert!(req.traceability.is_empty());
        assert!(req.restriction.is_none());
    }

    #[test]
    fn parses_requirement_with_all_advanced_traceability() {
        let req = get_requirement(
            "package p requirement x on y via z refines a specializes b is @@ text @@ requirement package",
        );
        assert_eq!(
            req.restriction.as_ref().map(|r| r.value.as_str()),
            Some("y")
        );
        assert_eq!(req.allocation.as_ref().map(|r| r.value.as_str()), Some("z"));
        assert_eq!(req.traceability.len(), 2);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.value, "b");
    }
}
