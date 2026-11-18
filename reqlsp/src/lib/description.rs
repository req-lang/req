use itertools::Itertools;
use reqlang::{
    Specification,
    node::{
        entity::{self, Arena},
        markup,
        path::Path,
        reference,
    },
    semantics::{path::Location, typing::expression::TypeKind},
};

use crate::{
    location::Target,
    tag::{TagSplit, tags_by_id},
};

pub fn kind_name(variant: &entity::Variant) -> &'static str {
    use entity::Variant::*;

    match variant {
        Part(_) => "part",
        Package(_) => "package",
        Requirement(_) => "requirement",
        Attribute(_) => "attribute",
        Type(_) => "type",
    }
}

fn type_text(paths: &Arena<Location>, typ: &TypeKind) -> String {
    match typ {
        TypeKind::Custom(entity) => paths[*entity].path.to_string(),
        TypeKind::Set(inner) => format!("Set({})", type_text(paths, inner)),
        other => format!("{other}"),
    }
}

fn markup_text(refs: &reference::Arena<Path>, markup: &markup::Markup) -> String {
    markup
        .unindented()
        .map(|item| match item {
            markup::Item::Text(text) => text,
            markup::Item::Indent(indent) => indent,
            markup::Item::Newline => "\n".to_string(),
            markup::Item::Reference(r) => refs[r].to_string(),
        })
        .collect()
}

pub fn render(spec: &Specification, target: &Target) -> String {
    let paths = &spec.analysis.paths;
    let types = &spec.analysis.types;

    match *target {
        Target::Entity(idx) => {
            let node = &spec.tree.entities[idx];
            let mut sections = vec![
                format!("**{}** `{}`", kind_name(&node.variant), node.meta.label),
                format!("`{}`", paths[idx].path),
            ];

            match &node.variant {
                entity::Variant::Attribute(attribute) => {
                    if let Ok(typ) = &spec.analysis.types[attribute.domain] {
                        let text = match typ {
                            TypeKind::Set(inner) => type_text(paths, inner),
                            typ => type_text(paths, typ),
                        };
                        sections.push(format!("type: `{}`", text));
                    }

                    if let Some(unit) = &attribute.unit {
                        sections.push(format!("unit: `{unit}`"));
                    }
                }
                entity::Variant::Requirement(requirement) => {
                    let variant = match requirement.variant {
                        entity::RequirementVariant::Formal(_) => "formal",
                        entity::RequirementVariant::Informal(_) => "informal",
                    };
                    sections.push(variant.to_string());
                }
                _ => {}
            }

            let tsplit = TagSplit::new(tags_by_id(&node.meta.tags));
            if !tsplit.valued.is_empty() {
                let list = tsplit
                    .valued
                    .into_iter()
                    .sorted_by_key(|(id, _)| *id)
                    .map(|(id, values)| {
                        let values = values.into_iter().sorted().join(", ");
                        format!("* {id}: {values}")
                    })
                    .join("\n");
                sections.push(list);
            }

            if let Some(comment) = &node.meta.comment {
                sections.push(markup_text(
                    &spec.tree.references,
                    &spec.tree.markups[*comment],
                ));
            }
            sections.join("\n\n")
        }
        Target::Expression(idx) => {
            let mut value = "expression".to_string();
            if let Ok(typ) = &types[idx] {
                value.push_str(&format!(": {}", type_text(paths, typ)));
            }
            value
        }
        Target::Identifier(idx) => {
            use reqlang::semantics::link::Target::*;

            let tree = &spec.tree;
            let links = &spec.analysis.links;
            let target = match &links.identifiers[idx] {
                None => "unresolved".to_string(),
                Some(Entity(idx)) => render(spec, &Target::Entity(*idx)),
                Some(Builtin(kind)) => format!("builtin `{kind:?}`"),
                Some(Enum(idx, member)) => {
                    let name = match &tree.entities[*idx].variant {
                        entity::Variant::Type(t) => t.members.get(*member).unwrap(),
                        _ => unreachable!(),
                    };
                    format!("member `{}` of {}", name, paths[*idx].path)
                }
                Some(Local(_)) => "local variable".to_string(),
            };
            target
        }
        Target::Reference(idx) => match spec.analysis.links.references[idx] {
            Some(tgt) => paths[tgt].path.to_string(),
            None => format!("unresolved"),
        },
    }
}
