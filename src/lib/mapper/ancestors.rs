use std::{collections::HashMap, ops::ControlFlow};

use crate::{
    children::Children,
    syntax::entity::{Entity, EntityKey},
    visitor::{Visitor, Walk, WalkCustom},
};

#[derive(Default, Debug)]
struct Mapper<'a> {
    current: Vec<EntityKey<'a>>,
    map: AncestorsMap<'a>,
}

impl<'a> Walk<&'a Entity> for Mapper<'a> {
    fn walk(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.visit(node)?;

        self.current.push(node.into());
        if let Some(children) = node.children() {
            for n in children {
                self.walk(n)?;
            }
        }
        self.current.pop();

        ControlFlow::Continue(())
    }
}

impl<'a> Visitor<&'a Entity> for Mapper<'a> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.map.insert(node.into(), self.current.clone());

        self.current.push(node.into());

        // TODO: Retablish expression and proper markup mapper

        self.current.pop();

        ControlFlow::Continue(())
    }
}

impl<'a> From<&'a Entity> for AncestorsMap<'a> {
    fn from(value: &'a Entity) -> Self {
        let mut ret = Mapper::default();
        _ = ret.walk(value);
        ret.map
    }
}

pub type AncestorsMap<'a> = HashMap<EntityKey<'a>, Vec<EntityKey<'a>>>;
