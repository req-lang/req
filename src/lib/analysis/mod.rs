use std::rc::Rc;

use crate::{
    path::Path,
    syntax::{
        self,
        entity::{EntityMeta, EntityVariant},
    },
};

pub type EntityRef = Rc<Entity>;

/// Entity tree where every reference is an `Rc` pointer to another
/// `ReferencedEntity`, and every child is an owned `ReferencedEntity`.
pub struct Entity {
    pub path: Path,
    pub meta: EntityMeta<Option<EntityRef>>,
    pub variant: EntityVariant<EntityRef, Option<EntityRef>>,
}

pub type Arena = Vec<Entity>;

pub struct ArenaBuilder {}
