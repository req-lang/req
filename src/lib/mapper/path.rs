use std::{collections::HashMap, ops::ControlFlow};

use crate::path::Path;
use crate::syntax::entity::{Entity, EntityKey};
use crate::{
    children::Children,
    visitor::{Visitor, Walk, WalkCustom},
};

#[derive(Default, Debug)]
struct Mapper<'n> {
    current: Vec<String>,
    map: PathMap<'n>,
}

impl<'n> Walk<&'n Entity> for Mapper<'n> {
    fn walk(&mut self, node: &'n Entity) -> ControlFlow<()> {
        self.current.push(node.meta.label.clone());
        self.visit(node)?;
        if let Some(children) = node.children() {
            for n in children {
                self.walk(n)?;
            }
        }
        self.current.pop();
        ControlFlow::Continue(())
    }
}

impl<'n> Visitor<&'n Entity> for Mapper<'n> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'n Entity) -> ControlFlow<()> {
        self.map.insert(node.into(), (&self.current).into());
        ControlFlow::Continue(())
    }
}

impl<'n> From<&'n Entity> for PathMap<'n> {
    fn from(node: &'n Entity) -> Self {
        let mut mapper = Mapper::default();
        let _ = mapper.walk(node);
        mapper.map
    }
}

pub type PathMap<'n> = HashMap<EntityKey<'n>, Path>;
