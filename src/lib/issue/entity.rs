use crate::{
    issue::{IssueCode, IssueLocation, IssueSeverity},
    syntax::entity::{EntityKey, Reference},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IssueKind<'a> {
    ImportNotFound(EntityKey<'a>),
    ReferenceNotFound(&'a Reference),
    AmbiguousReference(&'a Reference),
    RedefinedEntity(EntityKey<'a>),
    NotARequirement(EntityKey<'a>),
    CircularTrace(Vec<EntityKey<'a>>),
    NotPartOrAttribute(EntityKey<'a>),
    NonConstrainingRequirement,
    MissingRequirementBinding,
    SpecializationMismatch(EntityKey<'a>),
    RefinementMismatch(EntityKey<'a>),
    DerivationMismatch(EntityKey<'a>),
    NoDerivedAllocation(EntityKey<'a>),
    MissingDerivation,
    DuplicatedTrace(EntityKey<'a>),
    ConflictingTrace(EntityKey<'a>),
    SameAllocationAndConstraint,
    Unused,
    Unrestricted,
    Uncommented,
    UntypedAttribute,
    UnexpectedUnit,
    UnexpectedAttribute(EntityKey<'a>),
    UnexpectedForwardTrace,
    UnexpectedBrackwardTrace,
    MissingForwardTrace,
    MissingBackwardTrace,
    EmptyPackage,
    InhibitorForbidDerives,
    PossibleDuplicate(EntityKey<'a>),
}

impl IssueKind<'_> {
    pub const IMPORT_NOT_FOUND: IssueCode = IssueCode(1);
    pub const REFERENCE_NOT_FOUND: IssueCode = IssueCode(2);
    pub const AMBIGUOUS_REFERENCE: IssueCode = IssueCode(3);
    pub const REDEFINED_ENTITY: IssueCode = IssueCode(4);
    pub const NOT_A_REQUIREMENT: IssueCode = IssueCode(7);
    pub const CIRCULAR_TRACE: IssueCode = IssueCode(9);
    pub const NOT_PART_OR_ATTRIBUTE: IssueCode = IssueCode(10);
    pub const MISSING_REQUIREMENT_BINDING: IssueCode = IssueCode(11);
    pub const SPECIALIZATION_MISMATCH: IssueCode = IssueCode(12);
    pub const REFINEMENT_MISMATCH: IssueCode = IssueCode(13);
    pub const DERIVATION_MISMATCH: IssueCode = IssueCode(14);
    pub const NO_DERIVED_ALLOCATION: IssueCode = IssueCode(15);
    pub const MISSING_DERIVATION: IssueCode = IssueCode(16);
    pub const DUPLICATED_TRACE: IssueCode = IssueCode(17);
    pub const CONFLICTING_TRACE: IssueCode = IssueCode(18);
    pub const SAME_ALLOCATION_AND_CONSTRAINT: IssueCode = IssueCode(19);
    pub const UNUSED: IssueCode = IssueCode(20);
    pub const UNRESTRICTED: IssueCode = IssueCode(21);
    pub const UNCOMMENTED: IssueCode = IssueCode(22);
    pub const UNTYPED_ATTRIBUTE: IssueCode = IssueCode(23);
    pub const UNEXPECTED_UNIT: IssueCode = IssueCode(24);
    pub const UNEXPECTED_ATTRIBUTE: IssueCode = IssueCode(25);
    pub const UNEXPECTED_FORWARD_TRACE: IssueCode = IssueCode(26);
    pub const UNEXPECTED_BACKWARD_TRACE: IssueCode = IssueCode(27);
    pub const MISSING_FORWARD_TRACE: IssueCode = IssueCode(28);
    pub const MISSING_BACKWARD_TRACE: IssueCode = IssueCode(29);
    pub const EMPTY_PACKAGE: IssueCode = IssueCode(30);
    pub const INHIBITOR_FORBID_DERIVES: IssueCode = IssueCode(31);
    pub const POSSIBLE_DUPLICATE: IssueCode = IssueCode(32);
    pub const NON_CONSTRAINING_REQUIREMENT: IssueCode = IssueCode(33);
}

impl<'a> super::Issue<'a> for Issue<'a> {
    fn location(&self) -> super::IssueLocation<'a> {
        IssueLocation::Entity(self.location.clone())
    }

    fn message(&self) -> String {
        use IssueKind::*;
        let label = |key: &EntityKey| key.0.meta.label.to_string();
        match &self.kind {
            ImportNotFound(key) => format!("Import not found at {}", label(key)),
            ReferenceNotFound(_) => "Reference not found".to_string(),
            AmbiguousReference(_) => "Ambiguous reference".to_string(),
            RedefinedEntity(key) => format!("Redefined entity: {}", label(key)),
            NotARequirement(key) => format!("Expected a requirement at {}", label(key)),
            CircularTrace(_) => "Circular traceability".to_string(),
            NotPartOrAttribute(key) => format!("Part or attribute expected at {}", label(key)),
            MissingRequirementBinding => "Requirement is missing a constraint".to_string(),
            SpecializationMismatch(key) => format!(
                "Expected specialization to constraint the same entity as {}",
                label(key)
            ),
            RefinementMismatch(key) => format!(
                "Expected refinement to constraint a children entity from {}",
                label(key)
            ),
            DerivationMismatch(key) => format!(
                "Expected derivation to constraint the backward allocated entity by {}",
                label(key)
            ),
            NoDerivedAllocation(key) => format!(
                "Expected allocation in the derived requirement {}",
                label(key)
            ),
            MissingDerivation => {
                "Allocation of an entity without a deriving requirement".to_string()
            }
            DuplicatedTrace(key) => format!(
                "Same traceability link to {} is featured multiple times",
                label(key)
            ),
            ConflictingTrace(key) => format!(
                "Different traceability link for the same target {}",
                label(key)
            ),
            SameAllocationAndConstraint => "Traceability link is duplicated".to_string(),
            Unused => "Unused entity".to_string(),
            Unrestricted => "Unrestricted entity".to_string(),
            Uncommented => "Uncommented entity".to_string(),
            UntypedAttribute => "Expected the attribute to be typed".to_string(),
            UnexpectedUnit => "Units are expected only for numerical attributes".to_string(),
            UnexpectedAttribute(key) => format!("Unexpected attribute at {}", label(key)),
            UnexpectedForwardTrace => "Unexpected forward traceability".to_string(),
            UnexpectedBrackwardTrace => "Unexpected backward traceability".to_string(),
            MissingForwardTrace => "Missing forward traceability".to_string(),
            MissingBackwardTrace => "Missing backward traceability".to_string(),
            EmptyPackage => "Empty package".to_string(),
            NonConstrainingRequirement => {
                "Requirement does not expresses a boolean constraint".to_string()
            }
            InhibitorForbidDerives => {
                "Requirement is allocated but the inhibitor forbids derivation".to_string()
            }
            PossibleDuplicate(key) => {
                format!("The requirement might be a duplicate of {}", label(key))
            }
        }
    }

    fn code(&self) -> IssueCode {
        use IssueKind::*;
        match &self.kind {
            ImportNotFound(_) => IssueKind::IMPORT_NOT_FOUND,
            ReferenceNotFound(_) => IssueKind::REFERENCE_NOT_FOUND,
            AmbiguousReference(_) => IssueKind::AMBIGUOUS_REFERENCE,
            RedefinedEntity(_) => IssueKind::REDEFINED_ENTITY,
            NotARequirement(_) => IssueKind::NOT_A_REQUIREMENT,
            CircularTrace(_) => IssueKind::CIRCULAR_TRACE,
            NotPartOrAttribute(_) => IssueKind::NOT_PART_OR_ATTRIBUTE,
            MissingRequirementBinding => IssueKind::MISSING_REQUIREMENT_BINDING,
            SpecializationMismatch(_) => IssueKind::SPECIALIZATION_MISMATCH,
            RefinementMismatch(_) => IssueKind::REFINEMENT_MISMATCH,
            DerivationMismatch(_) => IssueKind::DERIVATION_MISMATCH,
            NoDerivedAllocation(_) => IssueKind::NO_DERIVED_ALLOCATION,
            MissingDerivation => IssueKind::MISSING_DERIVATION,
            DuplicatedTrace(_) => IssueKind::DUPLICATED_TRACE,
            ConflictingTrace(_) => IssueKind::CONFLICTING_TRACE,
            SameAllocationAndConstraint => IssueKind::SAME_ALLOCATION_AND_CONSTRAINT,
            Unused => IssueKind::UNUSED,
            Unrestricted => IssueKind::UNRESTRICTED,
            Uncommented => IssueKind::UNCOMMENTED,
            UntypedAttribute => IssueKind::UNTYPED_ATTRIBUTE,
            UnexpectedUnit => IssueKind::UNEXPECTED_UNIT,
            UnexpectedAttribute(_) => IssueKind::UNEXPECTED_ATTRIBUTE,
            UnexpectedForwardTrace => IssueKind::UNEXPECTED_FORWARD_TRACE,
            UnexpectedBrackwardTrace => IssueKind::UNEXPECTED_BACKWARD_TRACE,
            MissingForwardTrace => IssueKind::MISSING_FORWARD_TRACE,
            MissingBackwardTrace => IssueKind::MISSING_BACKWARD_TRACE,
            EmptyPackage => IssueKind::EMPTY_PACKAGE,
            InhibitorForbidDerives => IssueKind::INHIBITOR_FORBID_DERIVES,
            PossibleDuplicate(_) => IssueKind::POSSIBLE_DUPLICATE,
            NonConstrainingRequirement => IssueKind::NON_CONSTRAINING_REQUIREMENT,
        }
    }

    fn hint(&self) -> String {
        todo!()
    }

    fn severity(&self) -> super::IssueSeverity {
        use IssueKind::*;
        use IssueSeverity::*;

        match self.kind {
            ImportNotFound(_) => Critical,
            ReferenceNotFound(_) => Critical,
            AmbiguousReference(_) => Critical,
            RedefinedEntity(_) => Critical,
            NotARequirement(_) => Critical,
            NotPartOrAttribute(_) => Critical,
            UnexpectedAttribute(_) => Critical,
            InhibitorForbidDerives => Critical,
            NonConstrainingRequirement => Critical,

            MissingRequirementBinding => Severe,
            ConflictingTrace(_) => Severe,
            CircularTrace(_) => Severe,
            SpecializationMismatch(_) => Severe,
            RefinementMismatch(_) => Severe,
            DerivationMismatch(_) => Severe,

            UnexpectedUnit => Moderate,
            SameAllocationAndConstraint => Moderate,
            UnexpectedForwardTrace => Moderate,
            UnexpectedBrackwardTrace => Moderate,
            DuplicatedTrace(_) => Moderate,
            UntypedAttribute => Moderate,
            NoDerivedAllocation(_) => Moderate,
            Unrestricted => Moderate,

            Unused => Light,
            Uncommented => Light,
            MissingDerivation => Light,
            MissingForwardTrace => Light,
            MissingBackwardTrace => Light,
            EmptyPackage => Light,
            PossibleDuplicate(_) => Light,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue<'a> {
    pub location: EntityKey<'a>,
    pub kind: IssueKind<'a>,
}

impl<'a> Issue<'a> {
    pub fn new(location: EntityKey<'a>, kind: IssueKind<'a>) -> Self {
        Self { location, kind }
    }

    pub fn propagated(location: EntityKey<'a>, err: Issue<'a>) -> Self {
        Self {
            location,
            kind: err.kind,
        }
    }
}
