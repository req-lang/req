use crate::syntax::entity::Entity;

/// This module generates basic mapping between syntax node and node id.
/// Might be useful for later usage

impl<'a> From<&'a Entity> for NodeMap<'a> {
    fn from(node: &'a Entity) -> Self {
        let mut ret = Self::default();
        for n in node.iter() {
            ret.entities.insert(n.into(), n);
            for expression in n.expressions() {
                for e in expression.iter() {
                    ret.expressions.insert(e.into(), e);
                }
            }

            for m in n.markups() {
                ret.markups.insert(m.into(), m);
            }
        }
        ret
    }
}

#[derive(Default, Debug)]
pub struct NodeMap<'a> {
    pub entities: entity::NodeMap<'a>,
    pub expressions: expression::NodeMap<'a>,
    pub markups: markup::NodeMap<'a>,
}

pub mod entity {
    use std::collections::HashMap;

    use crate::syntax::entity::{Entity, EntityKey};

    pub type NodeMap<'a> = HashMap<EntityKey<'a>, &'a Entity>;
}

pub mod expression {
    use std::collections::HashMap;

    use crate::syntax::expression::{Expression, ExpressionKey};

    pub type NodeMap<'a> = HashMap<ExpressionKey<'a>, &'a Expression>;
}

pub mod markup {
    use std::collections::HashMap;

    use crate::syntax::markup::{Markup, MarkupKey};

    pub type NodeMap<'a> = HashMap<MarkupKey<'a>, &'a Markup>;
}
