use crate::{
    node::{arena, expression, markup::Markup, reference},
    utils::children::{Children, ChildrenIter, ChildrenMut, IntoChildren},
};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
pub type Arena<T> = arena::Arena<T, IdxMarker>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Node {
    pub meta: Meta,
    pub variant: Variant,
}

impl Node {
    pub fn new(meta: Meta, variant: Variant) -> Self {
        Self { meta, variant }
    }

    pub fn markups(&self) -> impl Iterator<Item = &Markup> {
        let mut ret = Vec::new();

        ret.extend(self.variant.markups());
        ret.extend(self.meta.comment.iter());
        ret.into_iter()
    }

    pub fn markups_mut(&mut self) -> impl Iterator<Item = &mut Markup> {
        let mut ret = Vec::new();

        ret.extend(self.variant.markups_mut());
        ret.extend(self.meta.comment.iter_mut());
        ret.into_iter()
    }
}

impl ChildrenIter<Idx> for Node {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Idx> {
        self.variant.children_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant {
    Part(Part),
    Attribute(Attribute),
    Package(Package),
    Requirement(Requirement),
    Type(Type),
}

pub trait Label {
    fn label(&self) -> String;
}

impl Children<Idx> for Variant {
    fn children(&self) -> Option<&Vec<Idx>> {
        match &self {
            Variant::Part(part) => Some(&part.children),
            Variant::Package(package) => Some(&package.children),
            _ => None,
        }
    }
}

impl ChildrenMut<Idx> for Variant {
    fn children_mut(&mut self) -> Option<&mut Vec<Idx>> {
        match self {
            Variant::Part(part) => Some(&mut part.children),
            Variant::Package(package) => Some(&mut package.children),
            _ => None,
        }
    }
}

impl IntoChildren<Idx> for Variant {
    fn into_children(self) -> Option<Vec<Idx>> {
        match self {
            Variant::Part(part) => Some(part.children),
            Variant::Package(package) => Some(package.children),
            _ => None,
        }
    }
}

impl ChildrenIter<Idx> for Variant {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Idx> {
        self.children()
            .into_iter()
            .map(|c| c.iter().copied())
            .flatten()
    }
}

impl Default for Variant {
    fn default() -> Self {
        Variant::Package(Package::default())
    }
}

impl Variant {
    pub fn part(&self) -> Option<&Part> {
        match self {
            Variant::Part(part) => Some(part),
            _ => None,
        }
    }

    pub fn into_package(self) -> Option<Package> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package(&self) -> Option<&Package> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package_mut(&mut self) -> Option<&mut Package> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn into_requirement(self) -> Option<Requirement> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn requirement(&self) -> Option<&Requirement> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn requirement_mut(&mut self) -> Option<&mut Requirement> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn attribute(&self) -> Option<&Attribute> {
        match self {
            Variant::Attribute(attribute) => Some(attribute),
            _ => None,
        }
    }

    pub fn attribute_mut(&mut self) -> Option<&mut Attribute> {
        match self {
            Variant::Attribute(attribute) => Some(attribute),
            _ => None,
        }
    }

    pub fn references(&self) -> impl Iterator<Item = reference::Idx> {
        let mut ret = Vec::new();

        match self {
            Variant::Package(package) => {
                ret.extend(&package.imports);
            }

            Variant::Part(part) => {
                ret.extend(&part.imports);
            }

            Variant::Requirement(requirement) => {
                ret.extend(requirement.restriction.iter());
                ret.extend(requirement.allocation.iter());
                ret.extend(requirement.traceability.iter().map(|t| &t.target));
            }

            _ => {}
        }

        ret.into_iter().copied()
    }

    pub fn expressions(&self) -> impl Iterator<Item = expression::Idx> {
        let mut ret = Vec::new();
        match self {
            Variant::Attribute(attribute) => ret.push(attribute.domain),
            Variant::Requirement(requirement) => match &requirement.variant {
                RequirementVariant::Formal(expr) => ret.push(*expr),
                RequirementVariant::Informal(_) => {}
            },
            _ => {}
        }
        ret.into_iter()
    }

    pub fn markups(&self) -> impl Iterator<Item = &Markup> {
        let mut ret = Vec::new();

        match self {
            Variant::Requirement(requirement) => match &requirement.variant {
                RequirementVariant::Informal(markup) => {
                    ret.push(markup);
                }
                RequirementVariant::Formal(_) => {}
            },
            _ => {}
        }

        ret.into_iter()
    }

    pub fn markups_mut(&mut self) -> impl Iterator<Item = &mut Markup> {
        let mut ret = Vec::new();

        match self {
            Variant::Requirement(requirement) => match &mut requirement.variant {
                RequirementVariant::Informal(markup) => {
                    ret.push(markup);
                }
                RequirementVariant::Formal(_) => {}
            },
            _ => {}
        }

        ret.into_iter()
    }

    pub fn imports(&self) -> Option<&Vec<reference::Idx>> {
        match &self {
            Variant::Part(part) => Some(&part.imports),
            Variant::Package(package) => Some(&package.imports),
            _ => None,
        }
    }

    pub fn imports_mut(&mut self) -> Option<&mut Vec<reference::Idx>> {
        match self {
            Variant::Part(part) => Some(&mut part.imports),
            Variant::Package(package) => Some(&mut package.imports),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Part {
    pub imports: Vec<reference::Idx>,
    pub children: Vec<Idx>,
}

impl Part {
    pub fn new(imports: Vec<reference::Idx>, children: Vec<Idx>) -> Self {
        Part { imports, children }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub domain: expression::Idx,
    pub unit: Option<String>,
}

impl Attribute {
    pub fn new(domain: expression::Idx) -> Self {
        Attribute { domain, unit: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Package {
    pub imports: Vec<reference::Idx>,
    pub children: Vec<Idx>,
}

impl Package {
    pub fn new(imports: Vec<reference::Idx>, children: Vec<Idx>) -> Self {
        Package { imports, children }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub restriction: Option<reference::Idx>,
    pub allocation: Option<reference::Idx>,
    pub traceability: Vec<TraceabilityLink>,
    pub inhibitor: TraceabilityInhibitor,
    pub variant: RequirementVariant,
}

impl From<RequirementVariant> for Requirement {
    fn from(variant: RequirementVariant) -> Self {
        Requirement {
            restriction: None,
            allocation: None,
            traceability: Vec::new(),
            inhibitor: TraceabilityInhibitor::default(),
            variant,
        }
    }
}

impl Requirement {
    pub fn informal(&self) -> Option<&Markup> {
        match &self.variant {
            RequirementVariant::Informal(markup) => Some(markup),
            _ => None,
        }
    }

    pub fn informal_mut(&mut self) -> Option<&mut Markup> {
        match &mut self.variant {
            RequirementVariant::Informal(markup) => Some(markup),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementVariant {
    Formal(expression::Idx),
    Informal(Markup),
}

impl Default for RequirementVariant {
    fn default() -> Self {
        Self::Informal(Markup::default())
    }
}

impl RequirementVariant {
    pub fn formal(&self) -> Option<&expression::Idx> {
        use RequirementVariant::*;

        match self {
            Formal(expr) => Some(expr),
            _ => None,
        }
    }
    pub fn informal(&self) -> Option<&Markup> {
        match self {
            RequirementVariant::Informal(markup) => Some(markup),
            _ => None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityInhibitor {
    pub top: bool,
    pub bottom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceabilityKind {
    Specialization,
    Refinement,
    Derivation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityLink {
    pub kind: TraceabilityKind,
    pub target: reference::Idx,
}

impl TraceabilityLink {
    pub fn new(kind: TraceabilityKind, target: reference::Idx) -> Self {
        TraceabilityLink { kind, target }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Type {
    pub members: Vec<String>,
}

impl From<Vec<String>> for Type {
    fn from(members: Vec<String>) -> Self {
        Type { members }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Meta {
    pub label: String,
    pub tags: Vec<Tag>,
    pub comment: Option<Markup>,
}

impl From<String> for Meta {
    fn from(value: String) -> Self {
        Self {
            label: value,
            tags: Default::default(),
            comment: Default::default(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    // TODO: To be renammed key
    pub id: String,
    pub value: Option<String>,
}

impl Tag {
    pub fn new(id: String, value: Option<String>) -> Self {
        Self { id, value }
    }
}
