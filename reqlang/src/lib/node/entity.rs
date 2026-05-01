use std::ops::{Index, IndexMut};

use crate::{
    node::{expression, markup},
    utils::children::{Children, ChildrenIter, ChildrenMut, IntoChildren},
};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Idx(usize);

impl From<Idx> for usize {
    fn from(value: Idx) -> Self {
        let Idx(val) = value;
        val
    }
}

impl From<usize> for Idx {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl<N, R, T, E, M> IndexMut<Idx> for Vec<Node<N, R, T, E, M>> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        let Idx(val) = index;
        self.index_mut(val)
    }
}

impl<N, R, T, E, M> Index<Idx> for Vec<Node<N, R, T, E, M>> {
    type Output = Node<N, R, T, E, M>;

    fn index(&self, index: Idx) -> &Self::Output {
        let Idx(val) = index;
        self.index(val)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Node<N, R, T, E, M, I = Idx, J = expression::Idx> {
    pub payload: N,
    pub meta: NodeMeta<M, R>,
    pub variant: NodeVariant<R, T, E, M, I, J>,
}

pub type NodeVariant<R, T, E, M, I = Idx, J = expression::Idx> =
    Variant<I, R, Vec<expression::Node<E, T, J>>, markup::Node<M, R>>;
pub type NodeMeta<M, R> = Meta<markup::Node<M, R>>;

impl<N, R, T, E, M, I, J> Node<N, R, T, E, M, I, J> {
    pub fn empty(meta: NodeMeta<M, R>, variant: NodeVariant<R, T, E, M, I, J>) -> Self
    where
        N: Default,
    {
        Self {
            payload: Default::default(),
            meta,
            variant,
        }
    }

    pub fn new(payload: N, meta: NodeMeta<M, R>, variant: NodeVariant<R, T, E, M, I, J>) -> Self {
        Self {
            payload,
            meta,
            variant,
        }
    }

    pub fn map<F, Q>(self, f: F) -> Node<Q, R, T, E, M, I, J>
    where
        F: FnOnce(N) -> Q,
    {
        Node {
            payload: f(self.payload),
            meta: self.meta,
            variant: self.variant,
        }
    }

    pub fn mapr<F, S>(self, mut f: F) -> Node<N, S, T, E, M, I, J>
    where
        F: FnMut(R) -> S,
    {
        Node {
            payload: self.payload,
            meta: self.meta.mapm(|m| m.mapr(&mut f)),
            variant: self.variant.mapr(&mut f).mapm(|m| m.mapr(&mut f)),
        }
    }

    pub fn mapt<F, U>(self, mut f: F) -> Node<N, R, U, E, M, I, J>
    where
        F: FnMut(T) -> U,
    {
        Node {
            payload: self.payload,
            meta: self.meta,
            variant: self
                .variant
                .mape(|e| e.into_iter().map(|e| e.mapr(&mut f)).collect()),
        }
    }

    pub fn mape<F, G>(self, mut f: F) -> Node<N, R, T, G, M, I, J>
    where
        F: FnMut(E) -> G,
    {
        Node {
            payload: self.payload,
            meta: self.meta,
            variant: self
                .variant
                .mape(|e| e.into_iter().map(|e| e.map(&mut f)).collect()),
        }
    }

    pub fn mapm<F, G>(self, mut f: F) -> Node<N, R, T, E, G, I, J>
    where
        F: FnMut(M) -> G,
    {
        Node {
            payload: self.payload,
            meta: self.meta.mapm(|m| m.map(&mut f)),
            variant: self.variant.mapm(|e| e.map(&mut f)),
        }
    }

    pub fn markups(&self) -> impl Iterator<Item = &markup::Node<M, R>> {
        let mut ret = Vec::new();

        ret.extend(self.variant.markups());
        ret.extend(self.meta.comment.iter());
        ret.into_iter()
    }

    pub fn markups_mut(&mut self) -> impl Iterator<Item = &mut markup::Node<M, R>> {
        let mut ret = Vec::new();

        ret.extend(self.variant.markups_mut());
        ret.extend(self.meta.comment.iter_mut());
        ret.into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant<N, R, E, M> {
    Part(Part<N, R>),
    Attribute(Attribute<E>),
    Package(Package<N, R>),
    Requirement(Requirement<R, E, M>),
}

pub trait Label {
    fn label(&self) -> String;
}

pub trait IntoVariable<N, R, E, M> {
    fn into_variant(self) -> Variant<N, R, E, M>;
}

pub trait VariableMut<N, R, E, M> {
    fn variant_mut(&mut self) -> &mut Variant<N, R, E, M>;
}

impl<N, R, E, M> Children<N> for Variant<N, R, E, M> {
    fn children(&self) -> Option<&Vec<N>> {
        match &self {
            Variant::Part(part) => Some(&part.children),
            Variant::Package(package) => Some(&package.children),
            _ => None,
        }
    }
}

impl<N, R, E, M> ChildrenMut<N> for Variant<N, R, E, M> {
    fn children_mut(&mut self) -> Option<&mut Vec<N>> {
        match self {
            Variant::Part(part) => Some(&mut part.children),
            Variant::Package(package) => Some(&mut package.children),
            _ => None,
        }
    }
}

impl<N, R, E, M> IntoChildren<N> for Variant<N, R, E, M> {
    fn into_children(self) -> Option<Vec<N>> {
        match self {
            Variant::Part(part) => Some(part.children),
            Variant::Package(package) => Some(package.children),
            _ => None,
        }
    }
}

// TODO: Remove that messy clone
impl<N: Clone, R, E, M> ChildrenIter<N> for Variant<N, R, E, M> {
    fn children_iter(&self) -> impl Iterator<Item = N> {
        self.children()
            .into_iter()
            .map(|c| c.iter())
            .flatten()
            .cloned()
    }
}

impl<N, R, E, M> Default for Variant<N, R, E, M> {
    fn default() -> Self {
        Variant::Package(Package::default())
    }
}

impl<N, R, E, M> Variant<N, R, E, M> {
    pub fn mapn<F, O>(self, mut f: F) -> Variant<O, R, E, M>
    where
        F: FnMut(N) -> O,
    {
        match self {
            Variant::Part(part) => Variant::Part(Part {
                imports: part.imports,
                children: part.children.into_iter().map(&mut f).collect(),
            }),
            Variant::Attribute(attribute) => Variant::Attribute(attribute),
            Variant::Package(package) => Variant::Package(Package {
                imports: package.imports,
                children: package.children.into_iter().map(&mut f).collect(),
            }),
            Variant::Requirement(requirement) => Variant::Requirement(requirement),
        }
    }

    pub fn mapr<F, S>(self, mut f: F) -> Variant<N, S, E, M>
    where
        F: FnMut(R) -> S,
    {
        match self {
            Variant::Part(part) => Variant::Part(Part {
                imports: part.imports.into_iter().map(&mut f).collect(),
                children: part.children,
            }),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                domain: attribute.domain,
                unit: attribute.unit,
            }),
            Variant::Package(package) => Variant::Package(Package {
                imports: package.imports.into_iter().map(&mut f).collect(),
                children: package.children,
            }),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                restriction: requirement.restriction.map(&mut f),
                allocation: requirement.allocation.map(&mut f),
                traceability: requirement
                    .traceability
                    .into_iter()
                    .map(|t| t.mapr(&mut f))
                    .collect(),
                inhibitor: requirement.inhibitor,
                variant: requirement.variant,
            }),
        }
    }

    pub fn mape<F, G>(self, f: F) -> Variant<N, R, G, M>
    where
        F: FnOnce(E) -> G,
    {
        match self {
            Variant::Part(part) => Variant::Part(part),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                domain: f(attribute.domain),
                unit: attribute.unit,
            }),
            Variant::Package(package) => Variant::Package(package),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                restriction: requirement.restriction,
                allocation: requirement.allocation,
                traceability: requirement.traceability,
                inhibitor: requirement.inhibitor,
                variant: match requirement.variant {
                    RequirementVariant::Formal(expr) => RequirementVariant::Formal(f(expr)),
                    RequirementVariant::Informal(markup) => RequirementVariant::Informal(markup),
                },
            }),
        }
    }

    pub fn mapm<F, O>(self, f: F) -> Variant<N, R, E, O>
    where
        F: FnOnce(M) -> O,
    {
        match self {
            Variant::Part(part) => Variant::Part(Part {
                imports: part.imports,
                children: part.children,
            }),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                domain: attribute.domain,
                unit: attribute.unit,
            }),
            Variant::Package(package) => Variant::Package(Package {
                imports: package.imports,
                children: package.children,
            }),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                restriction: requirement.restriction,
                allocation: requirement.allocation,
                traceability: requirement.traceability,
                inhibitor: requirement.inhibitor,
                variant: match requirement.variant {
                    RequirementVariant::Formal(expr) => RequirementVariant::Formal(expr),
                    RequirementVariant::Informal(markup) => RequirementVariant::Informal(f(markup)),
                },
            }),
        }
    }

    pub fn children(&self) -> Option<&Vec<N>> {
        match self {
            Variant::Part(part) => Some(&part.children),
            Variant::Package(package) => Some(&package.children),
            _ => None,
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<N>> {
        match self {
            Variant::Part(part) => Some(&mut part.children),
            Variant::Package(package) => Some(&mut package.children),
            _ => None,
        }
    }

    pub fn part(&self) -> Option<&Part<N, R>> {
        match self {
            Variant::Part(part) => Some(part),
            _ => None,
        }
    }

    pub fn into_package(self) -> Option<Package<N, R>> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package(&self) -> Option<&Package<N, R>> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package_mut(&mut self) -> Option<&mut Package<N, R>> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn into_requirement(self) -> Option<Requirement<R, E, M>> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn requirement(&self) -> Option<&Requirement<R, E, M>> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn requirement_mut(&mut self) -> Option<&mut Requirement<R, E, M>> {
        match self {
            Variant::Requirement(requirement) => Some(requirement),
            _ => None,
        }
    }

    pub fn attribute(&self) -> Option<&Attribute<E>> {
        match self {
            Variant::Attribute(attribute) => Some(attribute),
            _ => None,
        }
    }

    pub fn attribute_mut(&mut self) -> Option<&mut Attribute<E>> {
        match self {
            Variant::Attribute(attribute) => Some(attribute),
            _ => None,
        }
    }

    pub fn references(&self) -> impl Iterator<Item = &R> {
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

        ret.into_iter()
    }

    pub fn references_mut(&mut self) -> impl Iterator<Item = &mut R> {
        let mut ret = Vec::new();

        match self {
            Variant::Package(package) => {
                ret.extend(&mut package.imports);
            }

            Variant::Part(part) => {
                ret.extend(&mut part.imports);
            }

            Variant::Requirement(requirement) => {
                ret.extend(requirement.restriction.iter_mut());
                ret.extend(requirement.allocation.iter_mut());
                ret.extend(requirement.traceability.iter_mut().map(|t| &mut t.target));
            }
            _ => {}
        }

        ret.into_iter()
    }

    pub fn expressions(&self) -> impl Iterator<Item = &E> {
        let mut ret = Vec::new();
        match self {
            Variant::Attribute(attribute) => ret.push(&attribute.domain),
            Variant::Requirement(requirement) => match &requirement.variant {
                RequirementVariant::Formal(expr) => ret.push(expr),
                RequirementVariant::Informal(_) => {}
            },
            _ => {}
        }
        ret.into_iter()
    }

    pub fn expressions_mut(&mut self) -> impl Iterator<Item = &mut E> {
        let mut ret = Vec::new();
        match self {
            Variant::Attribute(attribute) => ret.push(&mut attribute.domain),
            Variant::Requirement(requirement) => match &mut requirement.variant {
                RequirementVariant::Formal(expr) => ret.push(expr),
                RequirementVariant::Informal(_) => {}
            },
            _ => {}
        }
        ret.into_iter()
    }

    pub fn markups(&self) -> impl Iterator<Item = &M> {
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

    pub fn markups_mut(&mut self) -> impl Iterator<Item = &mut M> {
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

    pub fn imports(&self) -> Option<&Vec<R>> {
        match &self {
            Variant::Part(part) => Some(&part.imports),
            Variant::Package(package) => Some(&package.imports),
            _ => None,
        }
    }

    pub fn imports_mut(&mut self) -> Option<&mut Vec<R>> {
        match self {
            Variant::Part(part) => Some(&mut part.imports),
            Variant::Package(package) => Some(&mut package.imports),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part<N, R> {
    pub imports: Vec<R>,
    pub children: Vec<N>,
}

impl<N, R> Default for Part<N, R> {
    fn default() -> Self {
        Self {
            imports: Default::default(),
            children: Default::default(),
        }
    }
}

impl<N, R> Part<N, R> {
    pub fn new(imports: Vec<R>, children: Vec<N>) -> Self {
        Part { imports, children }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<E> {
    pub domain: E,
    pub unit: Option<String>,
}

impl<E> Attribute<E> {
    pub fn new(domain: E) -> Self {
        Attribute { domain, unit: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package<N, R> {
    pub imports: Vec<R>,
    pub children: Vec<N>,
}

impl<N, R> Default for Package<N, R> {
    fn default() -> Self {
        Self {
            imports: Default::default(),
            children: Default::default(),
        }
    }
}

impl<N, R> Package<N, R> {
    pub fn new(imports: Vec<R>, children: Vec<N>) -> Self {
        Package { imports, children }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Requirement<R, E, M> {
    pub restriction: Option<R>,
    pub allocation: Option<R>,
    pub traceability: Vec<TraceabilityLink<R>>,
    pub inhibitor: TraceabilityInhibitor,
    pub variant: RequirementVariant<E, M>,
}

impl<R, E, M> From<RequirementVariant<E, M>> for Requirement<R, E, M> {
    fn from(variant: RequirementVariant<E, M>) -> Self {
        Requirement {
            restriction: None,
            allocation: None,
            traceability: Vec::new(),
            inhibitor: TraceabilityInhibitor::default(),
            variant,
        }
    }
}

impl<R, E, M> Requirement<R, E, M> {
    pub fn mapm<F, O>(self, f: F) -> Requirement<R, E, O>
    where
        F: FnOnce(M) -> O,
    {
        Requirement {
            restriction: self.restriction,
            allocation: self.allocation,
            traceability: self.traceability,
            inhibitor: self.inhibitor,
            variant: match self.variant {
                RequirementVariant::Informal(markup) => RequirementVariant::Informal(f(markup)),
                RequirementVariant::Formal(expr) => RequirementVariant::Formal(expr),
            },
        }
    }

    pub fn informal(&self) -> Option<&M> {
        match &self.variant {
            RequirementVariant::Informal(markup) => Some(markup),
            _ => None,
        }
    }

    pub fn informal_mut(&mut self) -> Option<&mut M> {
        match &mut self.variant {
            RequirementVariant::Informal(markup) => Some(markup),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementVariant<E, M> {
    Formal(E),
    Informal(M),
}

impl<E, M: Default> Default for RequirementVariant<E, M> {
    fn default() -> Self {
        Self::Informal(M::default())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityInhibitor {
    pub top: bool,
    pub bottom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceabilityKind {
    Specialization,
    Refinement,
    Derivation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceabilityLink<R> {
    pub kind: TraceabilityKind,
    pub target: R,
}

impl<R> TraceabilityLink<R> {
    pub fn new(kind: TraceabilityKind, target: R) -> Self {
        TraceabilityLink { kind, target }
    }

    pub fn mapr<S, F: FnOnce(R) -> S>(self, f: F) -> TraceabilityLink<S> {
        TraceabilityLink {
            kind: self.kind,
            target: f(self.target),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta<M> {
    pub label: String,
    pub tags: Vec<Tag>,
    pub comment: Option<M>,
}

impl<R> Default for Meta<R> {
    fn default() -> Self {
        Self {
            label: Default::default(),
            tags: Default::default(),
            comment: Default::default(),
        }
    }
}

impl<R> From<String> for Meta<R> {
    fn from(value: String) -> Self {
        Self {
            label: value,
            tags: Default::default(),
            comment: Default::default(),
        }
    }
}

impl<M> Meta<M> {
    pub fn mapm<F, O>(self, mut f: F) -> Meta<O>
    where
        F: FnMut(M) -> O,
    {
        Meta {
            label: self.label,
            tags: self.tags,
            comment: self.comment.map(&mut f),
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
