use reqlang::{
    node::entity::{Arena, Idx},
    semantics::{
        connection::{
            Connect,
            entity::{Connection, Neighbors},
        },
        link::{self, Resolve},
        path::Locate,
        typing::{Evaluate, TypeResult},
    },
    syntax,
};

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_valid_parsing_and_analysis(
    filename: &str,
) -> Arena<link::entity::Node<Neighbors<Connection<Idx>>, TypeResult, ()>> {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let result = syntax::parser::parse(&input);

    assert!(result.is_ok());

    let ast: Arena<_> = result
        .unwrap()
        .into_iter()
        .map(syntax::entity::Node::from)
        .collect();
    let arena = ast.locate().resolve().evaluate().connect();

    //
    // let issues: Vec<_> = NodeIter::new(connected.clone())
    //     .map(|n| IssueIterator::from(n.borrow()))
    //     .flatten()
    //     .collect();
    //
    // for issue in &issues {
    //     eprintln!("{:?}", issue);
    // }
    // assert!(issues.is_empty());

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
    fn forall() {
        assert_valid_parsing_and_analysis("forall.req");
    }

    #[test]
    fn exists() {
        assert_valid_parsing_and_analysis("exists.req");
    }

    #[test]
    fn select() {
        assert_valid_parsing_and_analysis("select.req");
    }

    #[test]
    fn when() {
        assert_valid_parsing_and_analysis("when.req");
    }

    #[test]
    fn branch() {
        assert_valid_parsing_and_analysis("branch.req");
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
