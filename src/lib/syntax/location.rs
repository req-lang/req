use std::{
    fmt::Display,
    ops::{Add, Range},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start.index..self.end.index
    }

    #[inline]
    pub fn merge(mut self, other: Self) -> Self {
        self.end = other.end;
        self
    }
}

impl From<(pest::error::LineColLocation, pest::error::InputLocation)> for Span {
    fn from(value: (pest::error::LineColLocation, pest::error::InputLocation)) -> Self {
        use pest::error::InputLocation;
        use pest::error::LineColLocation;

        let (line_col, location) = value;
        match (line_col, location) {
            (LineColLocation::Pos(p), InputLocation::Pos(index)) => Self {
                start: (p, index).into(),
                end: (p, index).into(),
            },

            (LineColLocation::Span(s, e), InputLocation::Span((sidx, eidx))) => Self {
                start: (s, sidx).into(),
                end: (e, eidx).into(),
            },

            (_, _) => unreachable!(),
        }
    }
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

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span<'_>) -> Self {
        Self {
            start: (value.start_pos().line_col(), value.start()).into(),
            end: (value.end_pos().line_col(), value.end()).into(),
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
    pub fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            index: 0,
        }
    }

    pub fn new(line: usize, column: usize, index: usize) -> Self {
        Self {
            line,
            column,
            index,
        }
    }

    pub fn next(pos: Position) -> Self {
        Self {
            line: pos.line,
            column: pos.column + 1,
            index: pos.index + 1,
        }
    }
}

impl From<((usize, usize), usize)> for Position {
    fn from(value: ((usize, usize), usize)) -> Self {
        let ((line, column), index) = value;
        Self {
            line,
            column,
            index,
        }
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            line: self.line + rhs.line - 1,
            column: self.column + rhs.column - 1,
            index: self.index + rhs.index,
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
