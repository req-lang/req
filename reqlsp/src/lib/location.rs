use reqlang::{
    node::{arena::idx::Idx, entity, expression, identifier, reference},
    semantics::link::Links,
    syntax::{self, location::SpanLike, parser::TokenArena},
};
use tower_lsp::lsp_types;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Identifier(identifier::Idx),
    Reference(reference::Idx),
    Expression(expression::Idx),
    Entity(entity::Idx),
}

impl Target {
    pub fn resolve(&self, links: &Links) -> Option<Resolved> {
        use reqlang::semantics::link::Target::*;

        match self {
            Target::Identifier(idx) => match links.identifiers[*idx]? {
                Entity(ex) | Enum(ex, _) => Some(Resolved::Entity(ex)),
                Local(ex) => Some(Resolved::Local(ex)),
                Builtin(_) => None,
            },
            Target::Reference(idx) => links.references[*idx].map(Resolved::Entity),
            Target::Expression(idx) => Some(Resolved::Local(*idx)),
            Target::Entity(idx) => Some(Resolved::Entity(*idx)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    Entity(entity::Idx),
    Local(expression::Idx),
}

impl Resolved {
    pub fn definition(&self, info: &syntax::parser::Info) -> Span {
        match self {
            Resolved::Local(ex) => info.expression.span_of(*ex).into(),
            Resolved::Entity(idx) => info
                .entities
                .name_of(*idx)
                .unwrap_or(info.entities.span_of(*idx))
                .into(),
        }
    }

    pub fn references(&self, info: &syntax::parser::Info, links: &Links) -> Vec<Span> {
        use reqlang::semantics::link::Target::*;

        match *self {
            Resolved::Entity(idx) => links
                .references
                .enumerate()
                .filter(|(_, target)| **target == Some(idx))
                .map(|(idx, _)| info.references.span_of(idx).into())
                .chain(
                    links
                        .identifiers
                        .enumerate()
                        .filter(|(_, target)| match target {
                            Some(Entity(tgt)) | Some(Enum(tgt, _)) => *tgt == idx,
                            _ => false,
                        })
                        .map(|(idx, _)| info.identifiers.span_of(idx).into()),
                )
                .collect(),

            Resolved::Local(idx) => links
                .identifiers
                .enumerate()
                .filter(|(_, target)| match target {
                    Some(Local(tgt)) => *tgt == idx,
                    _ => false,
                })
                .map(|(idx, _)| info.identifiers.span_of(idx).into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    pub fn rightmost<'a, T: SpanLike>(&self, spans: &'a [T]) -> Option<(usize, &'a T)> {
        let idx = spans
            .iter()
            .rposition(|s| Span::from(s.span()).start < *self);
        idx.map(|idx| (idx, &spans[idx]))
    }

    pub fn innermost<T: SpanLike, M>(&self, syntax: &TokenArena<T, M>) -> Option<(Idx<M>, Span)> {
        syntax
            .ranges()
            .indices()
            .map(|idx| (idx, syntax.span_of(idx)))
            .filter(|(_, span)| Span::from(*span).contains(*self))
            .min_by_key(|(_, span)| span.end.index - span.start.index)
            .map(|(idx, span)| (idx, span.into()))
    }

    pub fn target(&self, info: &syntax::parser::Info) -> Option<(Target, Span)> {
        if let Some((idx, span)) = self.innermost(&info.identifiers) {
            return Some((Target::Identifier(idx), span));
        }

        if let Some((idx, span)) = self.innermost(&info.references) {
            return Some((Target::Reference(idx), span));
        }

        if let Some((idx, span)) = self.innermost(&info.expression) {
            return Some((Target::Expression(idx), span));
        }

        if let Some((idx, span)) = self.innermost(&info.entities) {
            return Some((Target::Entity(idx), span));
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn empty(pos: Position) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    pub fn at_line_before(&self, pos: Position) -> Self {
        match self.start.line == pos.line {
            true => Self::new(self.start, pos),
            false => Self::empty(pos),
        }
    }
}

pub trait Contains<T> {
    fn contains(&self, other: T) -> bool;
    fn includes(&self, other: T) -> bool;
}

impl Contains<Position> for Span {
    fn contains(&self, other: Position) -> bool {
        self.start <= other && self.end > other
    }

    fn includes(&self, other: Position) -> bool {
        self.start <= other && self.end >= other
    }
}

impl Contains<Span> for Span {
    fn contains(&self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    fn includes(&self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

impl From<reqlang::syntax::location::Position> for Position {
    fn from(value: reqlang::syntax::location::Position) -> Self {
        Self {
            line: value.line as u32,
            column: value.column as u32,
        }
    }
}

impl From<reqlang::syntax::location::Span> for Span {
    fn from(value: reqlang::syntax::location::Span) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

impl From<lsp_types::Position> for Position {
    fn from(value: lsp_types::Position) -> Self {
        Self {
            line: value.line,
            column: value.character,
        }
    }
}

impl From<lsp_types::Range> for Span {
    fn from(value: lsp_types::Range) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(value: Position) -> Self {
        Self {
            line: value.line,
            character: value.column,
        }
    }
}

impl From<Span> for lsp_types::Range {
    fn from(value: Span) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}
