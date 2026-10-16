use reqlang::{
    issue::{Issue, Severity, connection_errors, expr_errors, path_issues, reference_errors},
    node::entity::{Arena, Idx},
    semantics::{
        connection::{
            Connect, Connected,
            entity::{Connection, Neighbors},
        },
        link::{self, Resolve},
        path::{self, Locate, Located, Location},
        typing::{Evaluate, TypeResult},
    },
    syntax::{self},
};

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_valid_parsing_and_analysis(
    filename: &str,
) -> Arena<link::entity::Node<Payload, TypeResult, ()>> {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let result = syntax::parser::parse(&input);

    assert!(result.is_ok());

    let ast: Arena<_> = result
        .unwrap()
        .into_iter()
        .map(syntax::entity::Node::from)
        .collect();

    let arena = ast.locate();
    let locations: Arena<_> = (&arena).into_iter().map(|n| n.payload.clone()).collect();

    let arena = arena.resolve().evaluate().connect();
    let arena: Arena<_> = arena
        .into_iter()
        .zip(locations)
        .map(|(node, loc)| node.map(|conns| Payload::new(loc, conns)))
        .collect();

    let issues: Arena<_> = (&arena)
        .into_iter()
        .map(|node| {
            path_issues(node)
                .chain(reference_errors(node))
                .chain(connection_errors(node))
                .chain(expr_errors(node))
                .map(Issue::from)
                .collect::<Vec<_>>()
        })
        .collect();

    for issue in &issues {
        eprintln!("{:?}", issue);
    }

    assert!(
        issues
            .into_iter()
            .flatten()
            .map(|issue| Severity::from(&issue))
            .filter(|sev| *sev == Severity::Critical)
            .next()
            .is_none()
    );

    arena
}

mod valid {
    use crate::assert_valid_parsing_and_analysis;

    #[test]
    fn simple() {
        assert_valid_parsing_and_analysis("simple.req");
    }

    #[test]
    fn formal() {
        assert_valid_parsing_and_analysis("formal.req");
    }

    #[test]
    fn arithmetics() {
        assert_valid_parsing_and_analysis("arithmetics.req");
    }

    #[test]
    fn logical() {
        assert_valid_parsing_and_analysis("logical.req");
    }

    #[test]
    fn temporal() {
        assert_valid_parsing_and_analysis("temporal.req");
    }

    #[test]
    fn set() {
        assert_valid_parsing_and_analysis("set.req");
    }

    #[test]
    fn interval() {
        assert_valid_parsing_and_analysis("interval.req");
    }

    #[test]
    fn r#enum() {
        assert_valid_parsing_and_analysis("enum.req");
    }

    #[test]
    fn comprehension() {
        assert_valid_parsing_and_analysis("comprehension.req");
    }

    #[test]
    fn keywords() {
        assert_valid_parsing_and_analysis("keywords.req");
    }

    #[test]
    fn when() {
        assert_valid_parsing_and_analysis("when.req");
    }

    #[test]
    fn aggregator() {
        assert_valid_parsing_and_analysis("aggregator.req");
    }

    #[test]
    fn traceability() {
        assert_valid_parsing_and_analysis("traceability.req");
    }
}

struct Payload {
    location: Location,
    neighbors: Neighbors<Connection<Idx>>,
}

impl Located for Payload {
    fn location(&self) -> &path::Location {
        &self.location
    }
}

impl Connected for Payload {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>> {
        &self.neighbors
    }
}

impl Payload {
    pub fn new(location: Location, neighbors: Neighbors<Connection<Idx>>) -> Self {
        Self {
            location,
            neighbors,
        }
    }
}
