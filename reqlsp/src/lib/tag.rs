use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use reqlang::node::entity::Tag;

pub type TagMap<'a> = HashMap<&'a String, HashSet<&'a Option<String>>>;

#[derive(Default, Debug, Clone)]
pub struct TagSplit<'a> {
    pub valued: HashMap<&'a String, Vec<&'a String>>,
    pub unvalued: HashSet<&'a String>,
}

impl<'a> TagSplit<'a> {
    pub fn new(map: TagMap<'a>) -> Self {
        let mut split = Self::default();
        for (id, values) in map.into_iter() {
            let mut valued = values.iter().filter_map(|x| x.as_ref()).peekable();
            if valued.peek().is_some() {
                split.valued.insert(id, valued.collect());
            }

            let mut unvalued = values.into_iter().filter(|x| x.is_none());
            if unvalued.next().is_some() {
                split.unvalued.insert(id);
            }
        }
        split
    }
}

pub fn tags_by_id(tags: &Vec<Tag>) -> TagMap<'_> {
    tags.iter()
        .map(|tag| (&tag.id, &tag.value))
        .into_grouping_map()
        .collect()
}
