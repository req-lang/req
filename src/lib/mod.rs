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

use diagnostic::{Issue, ResultErr};
use mapper::{
    ancestors::{self, AncestorsMap},
    id,
    path::{self, PathMap},
    requirement::TraceabilityMap,
};
use serde_derive::Serialize;
use syntax::entity::Entity;
use verifier::{
    definition::{self},
    reference::{self, ReferenceMap},
    typing::{self, TypeMap},
};
use visitor::Walk;

use crate::{
    detector::cycles,
    mapper::{
        requirement::UsageMap,
        subject::SubjectMap,
        tags::{self, TagMap},
    },
};

pub mod children;
pub mod detector;
pub mod diagnostic;
pub mod inverse;
pub mod iter;
pub mod mapper;
pub mod mock;
pub mod renderer;
pub mod syntax;
pub mod upsert;
pub mod verifier;
pub mod visitor;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Analysis {
    pub paths: PathMap,
    pub ancestors: AncestorsMap,
    pub references: ReferenceMap,
    pub types: TypeMap,
    pub traceabilities: TraceabilityMap,
    pub usages: UsageMap,
    pub tags: TagMap,
    pub diagnostics: Vec<Issue>,
}

impl From<&Entity> for Analysis {
    fn from(root: &Entity) -> Self {
        let paths = path::PathMap::from(root);
        let nodes_by_id = id::NodeMap::from(root);
        let ancestors = ancestors::AncestorsMap::from(root);

        let mut verifier = definition::Verifier::new(&paths);
        let _ = verifier.walk(root);
        let mut diagnostics = verifier.diagnostics;

        let mut verifier = reference::Verifier::new(&paths);
        let _ = verifier.walk(root);
        let (references, errs) = verifier.results.into_iter().map(ResultErr).collect();
        diagnostics.extend(errs);

        let mut verifier = typing::Verifier::new(&references, &nodes_by_id);
        let _ = verifier.prewalk(root);
        let _ = verifier.walk(root);
        let (types, errs) = verifier.results.into_iter().map(ResultErr).collect();
        diagnostics.extend(errs);

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
        diagnostics.extend(verifier.issues);

        let smap = SubjectMap::from(&rmap);
        let mut verifier = verifier::part::Verifier::new(&smap, &paths);
        let _ = verifier.walk(root);
        diagnostics.extend(verifier.issues);

        let mut verifier = verifier::attribute::Verifier::new(&smap, &types);
        let _ = verifier.walk(root);
        diagnostics.extend(verifier.issues);

        let mut mapper = tags::Mapper::default();
        let _ = mapper.walk(root);
        let tags = mapper.map;

        Self {
            paths,
            ancestors,
            references,
            types,
            traceabilities: rmap.forward,
            diagnostics,
            usages: smap.usages,
            tags,
        }
    }
}
