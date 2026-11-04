use reqlang::*;

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_isomorphic_formatter(filename: &str) {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let (expected, _) = syntax::parser::parse(&input).unwrap();

    let output = renderer::formatter::format_string(&expected, 0);
    let (actual, _) = syntax::parser::parse(&output).unwrap();

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
    fn interval() {
        assert_isomorphic_formatter("interval.req");
    }

    #[test]
    fn r#enum() {
        assert_isomorphic_formatter("enum.req");
    }

    #[test]
    fn comprehension() {
        assert_isomorphic_formatter("comprehension.req");
    }

    #[test]
    fn keywords() {
        assert_isomorphic_formatter("keywords.req");
    }

    #[test]
    fn when() {
        assert_isomorphic_formatter("when.req");
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
