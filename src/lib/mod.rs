// reqtool - Compiler and tooling for the req language
// Copyright (C) 2021-2026  Sami Dahoux
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// For commercial licensing, see COMMERCIAL.md

use mapper::{
    ancestors::{self, AncestorsMap},
    id,
    path::PathMap,
};
use resulterr::ResultErr;
use syntax::entity::Entity;
use verifier::{
    definition::{self},
    reference::{self, ReferenceMap},
    typing::{self, TypeMap},
};
use visitor::Walk;

use crate::{
    detector::cycles,
    issue::Issue,
    mapper::{
        requirement::RequirementMap,
        subject::SubjectMap,
        tags::{self, TagMap},
    },
};

pub mod children;
pub mod detector;
pub mod inverse;
pub mod issue;
pub mod iter;
pub mod key;
pub mod mapper;
pub mod mock;
pub mod path;
pub mod renderer;
pub mod resulterr;
pub mod syntax;
pub mod upsert;
pub mod verifier;
pub mod visitor;

pub struct Analysis<'a> {
    pub paths: PathMap<'a>,
    pub ancestors: AncestorsMap<'a>,
    pub references: ReferenceMap<'a>,
    pub types: TypeMap<'a>,
    pub requirements: RequirementMap<'a>,
    pub subjects: SubjectMap<'a>,
    pub tags: TagMap,
    pub diagnostics: Vec<Box<dyn Issue<'a> + 'a>>,
}

impl<'a> From<&'a Entity> for Analysis<'a> {
    fn from(root: &'a Entity) -> Self {
        let paths = mapper::path::PathMap::from(root);
        let nodes_by_id = id::NodeMap::from(root);
        let ancestors = ancestors::AncestorsMap::from(root);

        let mut verifier = definition::Verifier::new(&paths);
        let _ = verifier.walk(root);
        let mut diagnostics: Vec<Box<dyn Issue<'a> + 'a>> = verifier
            .diagnostics
            .into_iter()
            .map(|e| Box::new(e) as Box<dyn Issue<'a> + 'a>)
            .collect();

        let mut verifier = reference::Verifier::new(&paths);
        let _ = verifier.walk(root);
        let (references, errs): (_, Vec<_>) =
            verifier.entities.into_iter().map(ResultErr).collect();
        diagnostics.extend(errs.into_iter().map(|e| Box::new(e) as Box<dyn Issue<'a>>));

        let mut verifier = typing::Verifier::new(&references, &nodes_by_id);
        let _ = verifier.prewalk(root);
        let _ = verifier.walk(root);
        let (types, errs): (_, Vec<_>) = verifier.results.into_iter().map(ResultErr).collect();
        diagnostics.extend(
            errs.into_iter()
                .filter_map(|e: Option<_>| e)
                .map(|e| Box::new(e) as Box<dyn Issue<'a>>),
        );

        let mut mapper = mapper::requirement::Mapper::new(&references);
        let _ = mapper.walk(root);
        let rmap = mapper.map;

        let mut detector = cycles::traceability::Detector::new(&rmap.forward);
        detector.detect();
        let trace_cycles = detector.cycles;

        let mut verifier = verifier::requirement::Verifier::new(
            &nodes_by_id,
            &paths,
            &rmap,
            &trace_cycles,
            &types,
            &ancestors,
        );
        let _ = verifier.walk(root);
        diagnostics.extend(
            verifier
                .issues
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Issue<'a>>),
        );

        let smap = SubjectMap::from(&rmap);
        let mut verifier = verifier::part::Verifier::new(&smap, &paths);
        let _ = verifier.walk(root);
        diagnostics.extend(
            verifier
                .issues
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Issue<'a>>),
        );

        let mut verifier = verifier::attribute::Verifier::new(&smap, &types);
        let _ = verifier.walk(root);
        diagnostics.extend(
            verifier
                .issues
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Issue<'a>>),
        );

        let mut verifier = verifier::package::Verifier::new(&paths);
        let _ = verifier.walk(root);
        diagnostics.extend(
            verifier
                .issues
                .into_iter()
                .map(|e| Box::new(e) as Box<dyn Issue<'a>>),
        );

        let mut mapper = tags::Mapper::default();
        let _ = mapper.walk(root);
        let tags = mapper.map;

        Self {
            paths,
            ancestors,
            references,
            types,
            requirements: rmap,
            subjects: smap,
            diagnostics,
            tags,
        }
    }
}
