use crate::utils::children::{Children, ChildrenIter, ChildrenMut, IntoChildren};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variant<N, R, E, M> {
    Part(Part<N, R, M>),
    Attribute(Attribute<E, M>),
    Package(Package<N, R, M>),
    Requirement(Requirement<R, E, M>),
}

pub trait Label {
    fn label(&self) -> String;
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
    pub fn meta(&self) -> &Meta<M> {
        match self {
            Variant::Part(part) => &part.meta,
            Variant::Attribute(attribute) => &attribute.meta,
            Variant::Package(package) => &package.meta,
            Variant::Requirement(requirement) => &requirement.meta,
        }
    }

    pub fn meta_mut(&mut self) -> &mut Meta<M> {
        match self {
            Variant::Part(part) => &mut part.meta,
            Variant::Attribute(attribute) => &mut attribute.meta,
            Variant::Package(package) => &mut package.meta,
            Variant::Requirement(requirement) => &mut requirement.meta,
        }
    }

    pub fn mapn<F, O>(self, mut f: F) -> Variant<O, R, E, M>
    where
        F: FnMut(N) -> O,
    {
        match self {
            Variant::Part(part) => Variant::Part(Part {
                meta: part.meta,
                imports: part.imports,
                children: part.children.into_iter().map(&mut f).collect(),
            }),
            Variant::Attribute(attribute) => Variant::Attribute(attribute),
            Variant::Package(package) => Variant::Package(Package {
                meta: package.meta,
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
                meta: part.meta,
                imports: part.imports.into_iter().map(&mut f).collect(),
                children: part.children,
            }),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                meta: attribute.meta,
                domain: attribute.domain,
                unit: attribute.unit,
            }),
            Variant::Package(package) => Variant::Package(Package {
                meta: package.meta,
                imports: package.imports.into_iter().map(&mut f).collect(),
                children: package.children,
            }),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                meta: requirement.meta,
                restriction: requirement.restriction.map(&mut f),
                allocation: requirement.allocation.map(&mut f),
                traceability: requirement
                    .traceability
                    .into_iter()
                    .map(|t| TraceabilityLink {
                        kind: t.kind,
                        target: f(t.target),
                    })
                    .collect(),
                inhibitor: requirement.inhibitor,
                variant: requirement.variant,
            }),
        }
    }

    pub fn mape<F, G>(self, mut f: F) -> Variant<N, R, G, M>
    where
        F: FnMut(E) -> G,
    {
        match self {
            Variant::Part(part) => Variant::Part(part),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                meta: attribute.meta,
                domain: f(attribute.domain),
                unit: attribute.unit.map(&mut f),
            }),
            Variant::Package(package) => Variant::Package(package),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                meta: requirement.meta,
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

    pub fn mapm<F, O>(self, mut f: F) -> Variant<N, R, E, O>
    where
        F: FnMut(M) -> O,
    {
        match self {
            Variant::Part(part) => Variant::Part(Part {
                meta: part.meta.mapm(&mut f),
                imports: part.imports,
                children: part.children,
            }),
            Variant::Attribute(attribute) => Variant::Attribute(Attribute {
                meta: attribute.meta.mapm(&mut f),
                domain: attribute.domain,
                unit: attribute.unit,
            }),
            Variant::Package(package) => Variant::Package(Package {
                meta: package.meta.mapm(&mut f),
                imports: package.imports,
                children: package.children,
            }),
            Variant::Requirement(requirement) => Variant::Requirement(Requirement {
                meta: requirement.meta.mapm(&mut f),
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

    pub fn part(&self) -> Option<&Part<N, R, M>> {
        match self {
            Variant::Part(part) => Some(part),
            _ => None,
        }
    }

    pub fn into_package(self) -> Option<Package<N, R, M>> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package(&self) -> Option<&Package<N, R, M>> {
        match self {
            Variant::Package(package) => Some(package),
            _ => None,
        }
    }

    pub fn package_mut(&mut self) -> Option<&mut Package<N, R, M>> {
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

    pub fn attribute(&self) -> Option<&Attribute<E, M>> {
        match self {
            Variant::Attribute(attribute) => Some(attribute),
            _ => None,
        }
    }

    pub fn attribute_mut(&mut self) -> Option<&mut Attribute<E, M>> {
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

    pub fn markups(&self) -> impl Iterator<Item = &M> {
        let mut ret = Vec::new();

        match self {
            Variant::Requirement(requirement) => match &requirement.variant {
                RequirementVariant::Informal(markup) => {
                    ret.extend(requirement.meta.comment.iter());
                    ret.push(markup);
                }
                RequirementVariant::Formal(_) => {}
            },
            _ => {
                ret.extend(self.meta().comment.iter());
            }
        }

        ret.into_iter()
    }

    pub fn markups_mut(&mut self) -> impl Iterator<Item = &mut M> {
        let mut ret = Vec::new();

        match self {
            Variant::Requirement(requirement) => match &mut requirement.variant {
                RequirementVariant::Informal(markup) => {
                    ret.extend(requirement.meta.comment.iter_mut());
                    ret.push(markup);
                }
                RequirementVariant::Formal(_) => {}
            },
            _ => {
                ret.extend(self.meta_mut().comment.iter_mut());
            }
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
pub struct Part<N, R, M> {
    pub meta: Meta<M>,
    pub imports: Vec<R>,
    pub children: Vec<N>,
}

impl<N, R, M> Default for Part<N, R, M> {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            imports: Default::default(),
            children: Default::default(),
        }
    }
}

impl<N, R, M> Part<N, R, M> {
    pub fn new(imports: Vec<R>, children: Vec<N>) -> Self {
        Part {
            meta: Meta::default(),
            imports,
            children,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<E, M> {
    pub meta: Meta<M>,
    pub domain: E,
    pub unit: Option<E>,
}

impl<E, M> Attribute<E, M> {
    pub fn new(domain: E) -> Self {
        Attribute {
            meta: Meta::default(),
            domain,
            unit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package<N, R, M> {
    pub meta: Meta<M>,
    pub imports: Vec<R>,
    pub children: Vec<N>,
}

impl<N, R, M> Default for Package<N, R, M> {
    fn default() -> Self {
        Self {
            meta: Default::default(),
            imports: Default::default(),
            children: Default::default(),
        }
    }
}

impl<N, R, M> Package<N, R, M> {
    pub fn new(imports: Vec<R>, children: Vec<N>) -> Self {
        Package {
            meta: Meta::default(),
            imports,
            children,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Requirement<R, E, M> {
    pub meta: Meta<M>,
    pub restriction: Option<R>,
    pub allocation: Option<R>,
    pub traceability: Vec<TraceabilityLink<R>>,
    pub inhibitor: TraceabilityInhibitor,
    pub variant: RequirementVariant<E, M>,
}

impl<R, E, M> From<RequirementVariant<E, M>> for Requirement<R, E, M> {
    fn from(variant: RequirementVariant<E, M>) -> Self {
        Requirement {
            meta: Meta::default(),
            restriction: None,
            allocation: None,
            traceability: Vec::new(),
            inhibitor: TraceabilityInhibitor::default(),
            variant,
        }
    }
}

impl<R, E, M> Requirement<R, E, M> {
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
