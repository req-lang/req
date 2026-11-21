//! The codes of the issues
//!
//! A code identifies a kind of issue for good. `reqtool` and the language server show it, and the
//! reference documents each code. A code is never renumbered nor reused, even when the issue is no
//! longer reported.
//!
//! A code is written `E0`, the block of its [`Kind`] (one digit) and a number of three digits, like
//! `E04010`. The numbers of a block go in steps of ten, so that an issue that gets split in two can
//! take the codes next to its own.
//!
//! Adding an issue means adding its code to the registry below, and documenting it in the reference
//! (`lang-doc`, `source/reference/diagnostics.rst`).

use std::fmt;

/// How many codes a [`Kind`] can have
const BLOCK: u16 = 1000;

/// What an issue is about, the stage of the analysis that finds it
///
/// The discriminant is the block of the codes of the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// The source does not follow the grammar
    Syntax = 0,
    /// The name of an entity, `Path`s that are not unique
    Path = 1,
    /// A reference that points at nothing
    Reference = 2,
    /// The type of an expression
    Typing = 3,
    /// Where an entity is, and how requirements are linked to the entities and to each other
    Connection = 4,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Syntax => "syntax",
            Kind::Path => "path",
            Kind::Reference => "reference",
            Kind::Typing => "typing",
            Kind::Connection => "connection",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The code of an issue, `E04010` for example
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssueCode(u16);

impl IssueCode {
    /// The code of a kind of issue, `number` is its position in the block of `kind` (1 to 999)
    pub const fn new(kind: Kind, number: u16) -> Self {
        assert!(
            number > 0 && number < BLOCK,
            "the number of a code is in 1..=999"
        );

        Self(kind as u16 * BLOCK + number)
    }

    /// The digits of the code, `4010` for `E04010`
    pub const fn number(self) -> u16 {
        self.0
    }

    pub const fn kind(self) -> Kind {
        match self.0 / BLOCK {
            0 => Kind::Syntax,
            1 => Kind::Path,
            2 => Kind::Reference,
            3 => Kind::Typing,
            4 => Kind::Connection,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for IssueCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E0{:04}", self.0)
    }
}

macro_rules! registry {
    ($($(#[$doc:meta])* $name:ident = ($kind:ident, $number:literal);)*) => {
        $(
            $(#[$doc])*
            pub const $name: IssueCode = IssueCode::new(Kind::$kind, $number);
        )*

        /// Every code
        pub const ALL: &[IssueCode] = &[$($name),*];
    };
}

/// The registry of the codes
pub mod codes {
    use super::{IssueCode, Kind};

    registry! {
        /// The source does not follow the grammar
        SYNTAX_ERROR = (Syntax, 10);

        /// Two entities have the same path
        DUPLICATED_ENTITY = (Path, 10);

        /// A reference, in a text or in an expression, points at nothing
        UNRESOLVED_REFERENCE = (Reference, 10);

        UNEXPECTED_TYPE = (Typing, 10);
        NOT_A_SET = (Typing, 20);
        NOT_EVALUABLE = (Typing, 30);
        /// Reserved: it marks an expression that depends on a type error, and is never reported
        PROPAGATED_TYPE_ERROR = (Typing, 40);

        MISSING_REQUIREMENT_BINDING = (Connection, 10);
        MISSING_UPWARD_TRACE = (Connection, 20);
        MISSING_DOWNWARD_TRACE = (Connection, 30);
        INVALID_REQUIREMENT_PARENT = (Connection, 40);
        INVALID_PACKAGE_PARENT = (Connection, 50);
        INVALID_ATTRIBUTE_PARENT = (Connection, 60);
        UNRESTRICTED_SUBJECT = (Connection, 70);
        UNUSED_SUBJECT = (Connection, 80);
        NOT_A_PART_OR_ATTRIBUTE = (Connection, 90);
        NOT_A_REQUIREMENT = (Connection, 100);
        INHIBITOR_FORBID_DERIVES = (Connection, 110);
        MISSING_DERIVATION = (Connection, 120);
        DUPLICATED_TRACE = (Connection, 130);
        CONFLICTING_TRACE = (Connection, 140);
        SPECIALIZATION_MISMATCH = (Connection, 150);
        CYCLIC_TRACE = (Connection, 160);
        FORBIDDEN_UPWARD_TRACE = (Connection, 170);
        FORBIDDEN_DOWNWARD_TRACE = (Connection, 180);
        REFINEMENT_MISMATCH = (Connection, 190);
        NO_DERIVED_ALLOCATION = (Connection, 200);
        DERIVATION_MISMATCH = (Connection, 210);
        UNRELATED_DERIVATION = (Connection, 220);
        UNRELATED_RESTRICTION = (Connection, 230);
        SAME_ALLOCATION_AND_RESTRICTION = (Connection, 240);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{
        issue::Issue,
        node::{expression, reference},
        semantics::{
            connection::issue::{entity, reference as connection},
            path,
            typing::expression::{Issue as TypeIssue, TypeKind},
        },
    };

    #[test]
    fn a_code_is_written_with_its_block_and_four_digits() {
        assert_eq!(codes::SYNTAX_ERROR.to_string(), "E00010");
        assert_eq!(codes::DUPLICATED_ENTITY.to_string(), "E01010");
        assert_eq!(codes::UNEXPECTED_TYPE.to_string(), "E03010");
        assert_eq!(codes::MISSING_REQUIREMENT_BINDING.to_string(), "E04010");
        assert_eq!(codes::SAME_ALLOCATION_AND_RESTRICTION.to_string(), "E04240");
    }

    #[test]
    fn a_code_knows_its_kind() {
        assert_eq!(codes::SYNTAX_ERROR.kind(), Kind::Syntax);
        assert_eq!(codes::DUPLICATED_ENTITY.kind(), Kind::Path);
        assert_eq!(codes::UNRESOLVED_REFERENCE.kind(), Kind::Reference);
        assert_eq!(codes::NOT_A_SET.kind(), Kind::Typing);
        assert_eq!(codes::CYCLIC_TRACE.kind(), Kind::Connection);
    }

    #[test]
    fn the_number_of_a_code_is_the_digits_after_the_prefix() {
        assert_eq!(IssueCode::new(Kind::Connection, 10).number(), 4010);
        assert_eq!(IssueCode::new(Kind::Syntax, 10).number(), 10);
    }

    #[test]
    #[should_panic(expected = "1..=999")]
    fn the_number_zero_is_refused() {
        IssueCode::new(Kind::Typing, 0);
    }

    #[test]
    #[should_panic(expected = "1..=999")]
    fn a_number_over_the_block_is_refused() {
        IssueCode::new(Kind::Typing, 1000);
    }

    #[test]
    fn every_code_is_registered_once() {
        let distinct: BTreeSet<_> = codes::ALL.iter().collect();

        assert_eq!(distinct.len(), codes::ALL.len());
    }

    /// The codes of the issues of each family, as the analysis reports them
    fn reported() -> BTreeSet<IssueCode> {
        use TypeKind::{Boolean, Number};
        use connection::Issue as Net;
        use entity::Issue as Own;

        let own = [
            Own::MissingRequirementBinding,
            Own::MissingUpwardTrace,
            Own::MissingDownwardTrace,
            Own::InvalidRequirementParent,
            Own::InvalidPackageParent,
            Own::InvalidAttributeParent,
            Own::UnrestrictedSubject,
            Own::UnusedSubject,
        ]
        .iter()
        .map(Own::code);

        let net = [
            Net::NotAPartOrAttribute,
            Net::NotARequirement,
            Net::InhibitorForbidDerives,
            Net::MissingDerivation,
            Net::DuplicatedTrace,
            Net::ConflictingTrace,
            Net::SpecializationMismatch,
            Net::CyclicTrace,
            Net::ForbiddenUpwardTrace,
            Net::ForbiddenDownwardTrace,
            Net::RefinementMismatch,
            Net::NoDerivedAllocation,
            Net::DerivationMismatch,
            Net::UnrelatedDerivation,
            Net::UnrelatedRestriction,
            Net::SameAllocationAndRestriction,
        ]
        .iter()
        .map(Net::code);

        let typing = [
            TypeIssue::Poison,
            TypeIssue::NotASet(Number),
            TypeIssue::UnexpectedType(Number, Boolean),
            TypeIssue::NotEvaluable,
        ]
        .iter()
        .map(TypeIssue::code);

        own.chain(net)
            .chain(typing)
            .chain([path::Issue::Duplicated.code()])
            .chain([Issue::Reference(reference::Idx::default()).code()])
            .chain([codes::SYNTAX_ERROR])
            .collect()
    }

    #[test]
    fn the_registry_holds_exactly_the_codes_of_the_issues() {
        let registered: BTreeSet<_> = codes::ALL.iter().copied().collect();

        assert_eq!(registered, reported());
    }

    #[test]
    fn two_unresolved_targets_share_a_code() {
        let reference = Issue::Reference(reference::Idx::default());
        let identifier = Issue::Identifier(Default::default());

        assert_eq!(reference.code(), codes::UNRESOLVED_REFERENCE);
        assert_eq!(identifier.code(), codes::UNRESOLVED_REFERENCE);
    }

    #[test]
    fn an_issue_has_the_kind_of_its_code() {
        let typing = Issue::Type(TypeIssue::NotEvaluable, expression::Idx::default());
        let owned = Issue::Owned(entity::Issue::UnusedSubject);
        let network = Issue::Network(connection::Issue::CyclicTrace, reference::Idx::default());
        let duplicated = Issue::Path(path::Issue::Duplicated);

        assert_eq!(typing.kind(), Kind::Typing);
        assert_eq!(owned.kind(), Kind::Connection);
        assert_eq!(network.kind(), Kind::Connection);
        assert_eq!(duplicated.kind(), Kind::Path);
        assert_eq!(typing.code(), codes::NOT_EVALUABLE);
        assert_eq!(network.code(), codes::CYCLIC_TRACE);
    }
}
