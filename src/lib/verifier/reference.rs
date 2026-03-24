use std::{collections::HashMap, ops::ControlFlow};

use itertools::Itertools;

use crate::{
    children::{Children, ChildrenIter},
    issue::entity::Issue,
    mapper::path::PathMap,
    path::Path,
    syntax::{
        self,
        entity::{Entity, EntityKey, Reference, ReferenceKey},
        expression::{Expression, ExpressionKey},
    },
    visitor::{Visitor, Walk, WalkCustom},
};

pub struct Verifier<'a, 'b> {
    paths: &'a PathMap<'a>,
    inverse_paths: HashMap<&'a Path, &'a EntityKey<'a>>,
    parent: Vec<&'a Path>,
    local: HashMap<&'b str, ExpressionKey<'a>>,
    imports: Vec<Vec<&'a Path>>,
    current: Option<&'a Entity>,
    pub entities: HashMap<ReferenceKey<'a>, Result<EntityKey<'a>, Issue<'a>>>,
    pub expressions: HashMap<ReferenceKey<'a>, Result<ExpressionKey<'a>, Issue<'a>>>,
}

impl<'a, 'b> Walk<&'a Entity> for Verifier<'a, 'b> {
    fn walk(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.visit(node)?;

        if let Some(children) = node.children() {
            self.parent
                .push(&self.paths.get(&EntityKey::from(node)).unwrap());
            self.imports.push(
                node.imports()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| self.entities.get(&r.into()).unwrap().as_ref().ok())
                    .map(|found| self.paths.get(&EntityKey::from(*found)).unwrap())
                    .collect(),
            );
            for n in children {
                self.walk(n)?;
            }
            self.parent.pop();
            self.imports.pop();
        }

        ControlFlow::Continue(())
    }
}

impl<'a, 'b> Visitor<&'b Entity> for Verifier<'a, 'b> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.current = Some(node);

        for reference in node.references() {
            let found = self.find(reference);
            self.entities.insert(reference.into(), found);
        }

        for expression in node.expressions() {
            self.walk(expression)?;
        }

        for markup in node.markups() {
            for reference in markup.references() {
                let found = self.find(reference);
                self.entities.insert(reference.into(), found);
            }
        }

        ControlFlow::Continue(())
    }
}

impl<'a, 'b> Walk<&'b Expression> for Verifier<'a, 'b> {
    fn walk(&mut self, node: &'b Expression) -> ControlFlow<()> {
        self.visit(node)?;

        let old = match node.set_element() {
            None => None,
            Some(set) => self
                .local
                .insert(&set.variable, ExpressionKey::from(set.domain.as_ref())),
        };

        for child in node.children_iter() {
            self.walk(child)?;
        }

        if let Some(set) = node.set_element() {
            let label = &set.variable[..];
            match old {
                None => self.local.remove(label),
                Some(id) => self.local.insert(label, id),
            };
        }

        ControlFlow::Continue(())
    }
}

impl<'a, 'b> Visitor<&'b Expression> for Verifier<'a, 'b> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'b Expression) -> ControlFlow<()> {
        use syntax::expression::ExpressionVariant::*;

        match &node.variant {
            Identifier(ident) if !is_builtin(&ident.target.value) => {
                if let Some(id) = self.local.get(&ident.target.value[..]) {
                    self.expressions.insert(ident.target.into(), id.into());
                }

                let found = self.find(&ident.target);
                self.entities.insert(ident.target.into(), found);
            }

            _ => {}
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a, '_> {
    fn find(&self, reference: &'a Reference) -> Result<EntityKey<'a>, Issue> {
        use crate::issue::entity::IssueKind::{AmbiguousReference, ReferenceNotFound};

        let path = Path::from(reference.value.to_string());
        let as_path = std::iter::once(path);

        let parent = self.parent.last();
        let as_inner = parent.map(|p| p.appended(&reference.value)).into_iter();

        let imports = self.imports.last().unwrap();
        let as_imported = imports.iter().map(|p| p.merged(&reference.value));

        let mut hypotheses = as_inner
            .chain(as_path)
            .chain(as_imported)
            .filter_map(|p| self.inverse_paths.get(&Path::from(p)))
            .unique();

        let id = self.current.unwrap().into();
        match hypotheses.next() {
            None => Err(Issue::new(id, ReferenceNotFound(reference))),
            Some(found) => match hypotheses.next() {
                None => Ok(**found),
                Some(_) => Err(Issue::new(id, AmbiguousReference(reference))),
            },
        }
    }
}

impl<'a> Verifier<'a, '_> {
    pub fn new(paths: &'a PathMap) -> Self {
        let inverse_paths = paths.iter().map(|(id, path)| (path, id)).collect();
        Self {
            paths,
            inverse_paths,
            parent: Vec::new(),
            local: HashMap::new(),
            current: None,
            expressions: HashMap::new(),
            entities: HashMap::new(),
            imports: vec![Vec::new()],
        }
    }
}

pub type ReferenceMap<'a> = HashMap<ReferenceKey<'a>, EntityKey<'a>>;

#[inline]
pub fn is_builtin(val: &str) -> bool {
    match val {
        "real" | "integer" | "boolean" | "string" => true,
        _ => false,
    }
}
