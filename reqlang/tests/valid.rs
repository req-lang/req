use reqlang::{
    issue::IssueIterator,
    node::{self, NodeRef},
    semantics::{
        connection::{self, entity::Node},
        link, typing,
    },
    syntax,
    utils::iter::NodeIter,
};

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_valid_parsing_and_analysis(filename: &str) -> NodeRef<Node> {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let result = syntax::parser::parse(&input);

    assert!(result.is_ok());

    let ast = node::map(result.unwrap(), syntax::entity::Node::from);
    let resolved = link::resolve(ast);
    let typed = typing::evaluate(resolved.clone());
    let connected = connection::connect(resolved.clone());

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

    connected
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
