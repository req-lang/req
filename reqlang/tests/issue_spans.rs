use reqlang::{
    Specification,
    issue::{self, Issue},
    syntax,
};

/// The text each issue of the input is underlined on
fn underlined(input: &str, wanted: impl Fn(&Issue) -> bool) -> Vec<&str> {
    let (tree, info) = syntax::parser::parse(input);
    assert!(info.errors.is_empty(), "syntax errors: {:?}", info.errors);

    let entities = tree.entities.indices();
    let mut spec = Specification::from(tree);
    spec.diagnose();
    issue::issues(&spec)
        .into_iter()
        .zip(entities.into_iter())
        .flat_map(|(issues, entity)| {
            issues
                .into_iter()
                .filter(|issue| wanted(issue))
                .map(|issue| &input[issue.span(entity, &info).range()])
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The order of the issues is not specified
fn assert_same_items(found: Vec<&str>, expected: &[&str]) {
    assert_eq!(found.len(), expected.len(), "{found:?}");
    for item in expected {
        assert!(found.contains(item), "{item} not in {found:?}");
    }
}

/// The message and the range of each wanted issue of the input
fn underlined_ranges(
    input: &str,
    wanted: impl Fn(&Issue) -> bool,
) -> Vec<(String, std::ops::Range<usize>)> {
    let (tree, info) = syntax::parser::parse(input);
    assert!(info.errors.is_empty(), "syntax errors: {:?}", info.errors);

    let entities = tree.entities.indices();
    let mut spec = Specification::from(tree);
    spec.diagnose();
    issue::issues(&spec)
        .into_iter()
        .zip(entities.into_iter())
        .flat_map(|(issues, entity)| {
            issues
                .into_iter()
                .filter(|issue| wanted(issue))
                .map(|issue| (issue.to_string(), issue.span(entity, &info).range()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn unresolved_identifiers_are_underlined_on_the_identifier() {
    let input = "package p\n  requirement r is\n    (missing) + other\n  requirement\npackage";
    let found = underlined(input, |i| matches!(i, Issue::Identifier(_)));
    assert_same_items(found, &["missing", "other"]);
}

#[test]
fn unresolved_references_are_underlined_on_the_reference() {
    let input = "package p\n  requirement r refines nope is\n    @@\n      see {gone}\n    @@\n  requirement\npackage";
    let found = underlined(input, |i| matches!(i, Issue::Reference(_)));
    assert_same_items(found, &["gone", "nope"]);
}

#[test]
fn connection_issues_are_underlined_on_what_creates_the_connection() {
    // `q` is a requirement, neither a part nor an attribute, and it is both the restriction
    // and the allocation
    let input = "package p\n  requirement q is @@ x @@ requirement\n  requirement r on q via q is @@ y @@ requirement\npackage";
    let restriction = input.find("on q").unwrap() + 3;
    let allocation = input.find("via q").unwrap() + 4;

    let found = underlined_ranges(input, |i| matches!(i, Issue::Network(..)));
    for (message, start) in [
        ("part or attribute expected", restriction),
        ("part or attribute expected", allocation),
        (
            "allocation and restriction should be on different subjects",
            restriction,
        ),
        (
            "allocation and restriction should be on different subjects",
            allocation,
        ),
    ] {
        let range = start..start + 1;
        assert!(
            found.iter().any(|(m, r)| m == message && *r == range),
            "{message} at {start}: {found:?}"
        );
    }
}
