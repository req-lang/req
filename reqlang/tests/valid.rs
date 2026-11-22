use reqlang::{
    Specification,
    issue::{self},
    syntax::{self},
};

pub const PATH_ROOT: &str = "tests/data/valid";

fn assert_valid_parsing_and_analysis(filename: &str) -> Specification {
    let path = std::path::PathBuf::from(PATH_ROOT).join(filename);
    let input = std::fs::read_to_string(path).unwrap();
    let (tree, info) = syntax::parser::parse(&input);
    let errors = info.errors;

    assert!(errors.is_empty(), "syntax errors: {:?}", errors);

    let mut spec = Specification::from(tree);
    spec.diagnose();
    let issues: Vec<_> = issue::issues(&spec).into_iter().flatten().collect();

    assert!(issues.is_empty(), "issues: {:?}", issues);

    spec
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
