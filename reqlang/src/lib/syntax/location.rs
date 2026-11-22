use std::{
    fmt::Display,
    ops::{Add, Range},
    str::CharIndices,
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

pub trait SpanLike {
    fn span(&self) -> Span;
}

impl SpanLike for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start.index..self.end.index
    }
}

/// Smallest span containing all of `spans`. The spans may be nested (a parent comes before its
/// children and can end after the last of them), so the last one is not necessarily the one
/// that ends last.
pub fn hull(spans: impl Iterator<Item = Span>) -> Span {
    spans
        .reduce(|hull, span| Span {
            start: if span.start.index < hull.start.index {
                span.start
            } else {
                hull.start
            },
            end: if span.end.index > hull.end.index {
                span.end
            } else {
                hull.end
            },
        })
        .unwrap_or_default()
}

impl Add<Position> for Span {
    type Output = Self;

    fn add(self, rhs: Position) -> Self::Output {
        Self {
            start: self.start + rhs,
            end: self.end + rhs,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, index: usize) -> Self {
        Self {
            line,
            column,
            index,
        }
    }
}

impl From<((usize, usize), usize)> for Position {
    fn from(value: ((usize, usize), usize)) -> Self {
        let ((line, column), index) = value;
        Self {
            line: line - 1,
            column: column - 1,
            index,
        }
    }
}

/// Translates a position relative to the start of a text (`self`) by the position of that
/// text in the file (`rhs`). Only the first line starts at the column of the text, the next
/// ones start at the beginning of their line.
impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            line: self.line + rhs.line,
            column: if self.line == 0 {
                self.column + rhs.column
            } else {
                self.column
            },
            index: self.index + rhs.index,
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

pub struct PositionIterator<'a> {
    chars: CharIndices<'a>,
    current: Position,
    input: &'a str,
    offset: usize,
}

impl<'a> PositionIterator<'a> {
    pub fn current(&self) -> Position {
        self.current
    }

    pub fn input(&self) -> &'a str {
        self.input
    }
}

impl<'a> Iterator for PositionIterator<'a> {
    type Item = (Position, char);

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, c) = self.chars.next()?;
        self.current.index = self.offset + idx;

        let ret = (self.current, c);
        self.current.index += c.len_utf8();
        match c {
            '\n' => {
                self.current.line += 1;
                self.current.column = 0;
            }

            _ => {
                self.current.column += c.len_utf16();
            }
        }

        return Some(ret);
    }
}

pub trait IntoPosition<'a> {
    fn positioned(self) -> PositionIterator<'a>;
}

impl<'a> IntoPosition<'a> for CharIndices<'a> {
    fn positioned(self) -> PositionIterator<'a> {
        PositionIterator {
            input: self.as_str(),
            chars: self,
            current: Position::default(),
            offset: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_counts_bytes_and_column_counts_utf16_units() {
        let positions: Vec<_> = "é😀a\nb😀c"
            .char_indices()
            .positioned()
            .map(|(p, _)| (p.line, p.column, p.index))
            .collect();
        assert_eq!(
            positions,
            [
                (0, 0, 0),
                (0, 1, 2),
                (0, 3, 6),
                (0, 4, 7),
                (1, 0, 8),
                (1, 1, 9),
                (1, 3, 13)
            ]
        );
    }

    #[test]
    fn current_is_after_the_last_char() {
        let mut positions = "😀".char_indices().positioned();
        positions.next();
        assert_eq!(positions.current(), Position::new(0, 2, 4));
    }
}
