use crate::node::{entity, expression, identifier, markup, path::Path, reference};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub entities: entity::Arena<entity::Node>,
    pub expressions: expression::Arena<expression::Node>,
    pub references: reference::Arena<Path>,
    pub identifiers: identifier::Arena<Path>,
    pub markups: markup::Arena<markup::Markup>,
}

impl Tree {
    pub fn root(&self) -> entity::Idx {
        self.entities.last_idx()
    }
}
