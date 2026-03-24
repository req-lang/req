use std::ops::ControlFlow;

use crate::{
    issue::entity::{Issue, IssueKind},
    mapper::subject::SubjectMap,
    syntax::{
        entity::{Entity, EntityVariant},
        expression::ExpressionKey,
    },
    verifier::typing::{TypeKind, TypeMap},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    types: &'a TypeMap<'a>,
    smap: &'a SubjectMap<'a>,
    pub issues: Vec<Issue<'a>>,
}

impl<'n> Visitor<&'n Entity> for Verifier<'n> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'n Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;

        if let Attribute(attribute) = &node.variant {
            let mut issues = Vec::new();

            if let None = self.smap.usages.get(&node.into()) {
                issues.push(Unused);
            }

            if let None = self.smap.restrictions.get(&node.into()) {
                issues.push(Unrestricted);
            }

            if let None = node.meta.comment {
                issues.push(Uncommented);
            }

            match self.types.get(&ExpressionKey::from(&attribute.domain)) {
                Some(TypeKind::Set(typ)) => {
                    let mut inner = *typ.clone();
                    loop {
                        match inner {
                            TypeKind::Set(typ) => inner = *typ,
                            typ => {
                                inner = typ;
                                break;
                            }
                        }
                    }

                    match inner {
                        TypeKind::Number => {}
                        TypeKind::Undefined => issues.push(UntypedAttribute),
                        _ if attribute.unit.is_some() => issues.push(UnexpectedUnit),
                        _ => {}
                    }
                }
                Some(TypeKind::Undefined) | None => issues.push(UntypedAttribute),
                Some(_) => issues.push(UntypedAttribute), // TODO: Check if better error possible
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.into(), i));
            self.issues.extend(iss);
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a> {
    pub fn new(smap: &'a SubjectMap, types: &'a TypeMap) -> Self {
        Self {
            smap,
            types,
            issues: Vec::new(),
        }
    }
}
