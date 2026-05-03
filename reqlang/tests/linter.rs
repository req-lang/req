use reqlang::{node::entity::Arena, *};

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_isomorphic_formatter(filename: &str) {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let expected: Arena<_> = syntax::parser::parse(&input)
        .unwrap()
        .into_iter()
        .map(syntax::entity::Node::from)
        .collect();

    let output = renderer::formatter::format_string(&expected, 0);
    let actual: Arena<_> = syntax::parser::parse(&output)
        .unwrap()
        .into_iter()
        .map(syntax::entity::Node::from)
        .collect();

    if actual != expected || input != output {
        eprintln!("*** INPUT ***\n{}\n\n", input);
        eprintln!("*** OUTPUT ***\n{}\n", output);
    }

    assert!(actual == expected);
    assert!(input == output);
}

mod formatter {
    use crate::assert_isomorphic_formatter;

    #[test]
    fn simple() {
        assert_isomorphic_formatter("simple.req");
    }

    #[test]
    fn formal() {
        assert_isomorphic_formatter("formal.req");
    }

    #[test]
    fn arithmetics() {
        assert_isomorphic_formatter("arithmetics.req");
    }

    #[test]
    fn logical() {
        assert_isomorphic_formatter("logical.req");
    }

    #[test]
    fn temporal() {
        assert_isomorphic_formatter("temporal.req");
    }

    #[test]
    fn set() {
        assert_isomorphic_formatter("set.req");
    }

    #[test]
    fn forall() {
        assert_isomorphic_formatter("forall.req");
    }

    #[test]
    fn exists() {
        assert_isomorphic_formatter("exists.req");
    }

    #[test]
    fn select() {
        assert_isomorphic_formatter("select.req");
    }

    #[test]
    fn when() {
        assert_isomorphic_formatter("when.req");
    }

    #[test]
    fn branch() {
        assert_isomorphic_formatter("branch.req");
    }

    #[test]
    fn aggregator() {
        assert_isomorphic_formatter("aggregator.req");
    }

    #[test]
    fn traceability() {
        assert_isomorphic_formatter("traceability.req");
    }

    #[test]
    fn tricky() {
        assert_isomorphic_formatter("tricky_format.req");
    }
}
