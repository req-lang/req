use std::{collections::HashMap, ops::ControlFlow};

use crate::path::Path;
use crate::syntax::entity::{Entity, EntityKey};
use crate::{
    children::Children,
    visitor::{Visitor, Walk, WalkCustom},
};

#[derive(Default, Debug)]
struct Mapper<'a> {
    current: Vec<String>,
    map: PathMap<'a>,
}

impl<'a> Walk<&'a Entity> for Mapper<'a> {
    fn walk(&mut self, node: &'a Entity) -> ControlFlow<()> {
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

impl<'a> Visitor<&'a Entity> for Mapper<'a> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.map.insert(node.into(), (&self.current).into());
        ControlFlow::Continue(())
    }
}

impl<'a> From<&'a Entity> for PathMap<'a> {
    fn from(node: &'a Entity) -> Self {
        let mut mapper = Mapper::default();
        let _ = mapper.walk(node);
        mapper.map
    }
}

pub type PathMap<'a> = HashMap<EntityKey<'a>, Path>;
