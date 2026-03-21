use std::collections::{HashMap, HashSet};

use crate::{inverse::Inverse, mapper::requirement::RequirementMap, syntax::NodeId};

pub type RestrictionMap = HashMap<NodeId, HashSet<NodeId>>;
pub type AllocationMap = HashMap<NodeId, HashSet<NodeId>>;
pub type UsageMap = HashMap<NodeId, HashSet<NodeId>>;

pub struct SubjectMap {
    pub restrictions: RestrictionMap,
    pub allocations: AllocationMap,
    pub usages: UsageMap,
}

impl From<&RequirementMap> for SubjectMap {
    fn from(value: &RequirementMap) -> Self {
        Self {
            restrictions: value.restrictions.inversed(),
            allocations: value.allocations.inversed(),
            usages: value.usages.inversed(),
        }
    }
}
