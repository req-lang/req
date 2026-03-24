use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::subject::SubjectMap,
    syntax::entity::{Entity, EntityVariant},
    verifier::typing::{TypeKind, TypeMap},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    types: &'a TypeMap<'a>,
    smap: &'a SubjectMap<'a>,
    pub issues: Vec<Issue<'a>>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
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

            match self.types.get(&attribute.domain.into()) {
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
                Some(t) => issues.push(NotASet(t.clone())),
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
