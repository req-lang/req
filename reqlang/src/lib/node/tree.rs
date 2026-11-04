use crate::node::{entity, expression, identifier, path::Path, reference};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub entities: entity::Arena<entity::Node>,
    pub expressions: expression::Arena<expression::Node>,
    pub references: reference::Arena<Path>,
    pub identifiers: identifier::Arena<Path>,
}

#[derive(Default, Debug, Clone)]
pub struct Draft {
    pub entities: Vec<entity::Node>,
    pub expressions: Vec<expression::Node>,
    pub references: Vec<Path>,
    pub identifiers: Vec<Path>,
}

impl From<Draft> for Tree {
    fn from(draft: Draft) -> Self {
        Tree {
            entities: draft.entities.into(),
            expressions: draft.expressions.into(),
            references: draft.references.into(),
            identifiers: draft.identifiers.into(),
        }
    }
}

impl Tree {
    pub fn root(&self) -> entity::Idx {
        self.entities.last_idx()
    }
}
