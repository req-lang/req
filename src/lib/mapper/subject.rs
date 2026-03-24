use std::collections::{HashMap, HashSet};

use crate::{inverse::Inverse, mapper::requirement::RequirementMap, syntax::entity::EntityKey};

pub type RestrictionMap<'a> = HashMap<EntityKey<'a>, HashSet<EntityKey<'a>>>;
pub type AllocationMap<'a> = HashMap<EntityKey<'a>, HashSet<EntityKey<'a>>>;
pub type UsageMap<'a> = HashMap<EntityKey<'a>, HashSet<EntityKey<'a>>>;

#[derive(Debug, Clone, Default)]
pub struct SubjectMap<'a> {
    pub restrictions: RestrictionMap<'a>,
    pub allocations: AllocationMap<'a>,
    pub usages: UsageMap<'a>,
}

impl<'a> From<&'a RequirementMap<'a>> for SubjectMap<'a> {
    fn from(value: &'a RequirementMap) -> Self {
        Self {
            restrictions: value.restrictions.inversed(),
            allocations: value.allocations.inversed(),
            usages: value.usages.inversed(),
        }
    }
}
