use pretty_assertions::assert_eq;
use reqlang::{Specification, exporter, syntax};
use unindent::unindent;

/// The export of what the parser makes of the input, as the playground does it, syntax errors or not
fn export(input: &str) -> String {
    let (tree, _) = syntax::parser::parse(&unindent(input));
    exporter::html::render_string(&Specification::from(tree))
}

#[test]
fn the_structure_is_headings_one_level_deeper_at_each_nesting() {
    let html = export(
        "
        package Sensor_node
          part Radio
            let report_period in real
          part
          requirement Regular_reporting
            on Radio
          is
            @@
            It reports.
            @@
          requirement
        package
        ",
    );

    assert_eq!(
        html,
        unindent(
            r#"
            <h2 id="pv-3">Sensor node</h2>
            <h3 id="pv-2">Regular reporting</h3>
            <p>It reports.</p>
            <h3 id="pv-1">Radio</h3>
            <h4 id="pv-0">report period</h4>
            "#
        )
    );
}

#[test]
fn a_reference_is_a_link_to_the_heading_of_its_entity() {
    let html = export(
        "
        package S
          part Radio
          part
          requirement R
            on Radio
          is
            @@
            The {Radio} shall report. See {Radio} and {Nowhere}.
            @@
          requirement
        package
        ",
    );

    assert!(
        html.contains(r##"<p>The <a href="#pv-0">Radio</a> shall report. See <a href="#pv-0">Radio</a> and {Nowhere}.</p>"##),
        "{html}"
    );
    assert!(html.contains(r#"<h3 id="pv-0">Radio</h3>"#), "{html}");
}

#[test]
fn a_type_has_no_heading_so_a_reference_to_it_is_not_a_link() {
    let html = export(
        "
        package S
          type T has
            A |
            B
          type
          requirement R
          is
            @@
            Uses {T}.
            @@
          requirement
        package
        ",
    );

    assert!(html.contains("<p>Uses {T}.</p>"), "{html}");
    assert!(!html.contains("href"), "{html}");
}

#[test]
fn a_formal_expression_is_code_laid_out_as_the_formatter_does() {
    let html = export(
        "
        package S
          part P
            let x in real
          part
          requirement R
            on P::x
          is
            when
              P::x > 3.0 then P::x < 5.0,
              otherwise P::x = 0
            end
          requirement
        package
        ",
    );

    assert!(
        html.contains(&unindent(
            r#"
            <pre><code class="language-req">when
              P::x &gt; 3.0 then P::x &lt; 5.0,
              otherwise P::x = 0
            end</code></pre>
            "#
        )),
        "{html}"
    );
}

#[test]
fn the_comment_of_a_requirement_follows_it_in_an_aside() {
    let html = export(
        "
        package S
          part P
          part
          @@
          Why: **safety**.
          @@
          requirement R
            on P
          is
            @@
            It holds.
            @@
          requirement
        package
        ",
    );

    assert!(
        html.contains(&unindent(
            "
            <p>It holds.</p>
            <aside class=\"comment\">
            <p>Why: <strong>safety</strong>.</p>
            </aside>
            "
        )),
        "{html}"
    );
}

#[test]
fn informal_text_is_markdown() {
    let html = export(
        "
        package S
          requirement R
          is
            @@
            It **shall**:

            - start
            - stop
            @@
          requirement
        package
        ",
    );

    assert!(html.contains("<strong>shall</strong>"), "{html}");
    assert!(html.contains("<li>start</li>"), "{html}");
}

#[test]
fn text_that_looks_like_html_or_runs_code_is_only_text() {
    let html = export(
        r#"
        package S
          requirement R
          is
            @@
            <script>alert(1)</script> <img src=x onerror=alert(1)>

            [run](javascript:alert(1)) ![remote](https://tracker.test/pixel.png)

            <div>block</div>
            @@
          requirement
        package
        "#,
    );

    for forbidden in ["<script", "<img", "<div", "href=\"javascript", " src=\""] {
        assert!(!html.contains(forbidden), "{forbidden} in {html}");
    }
    assert!(html.contains("&lt;script&gt;"), "{html}");
}

#[test]
fn a_specification_with_syntax_errors_is_exported_as_far_as_it_was_understood() {
    let html = export(
        "
        package System
          part Engine
          part
          part
        package
        ",
    );

    assert!(html.contains(r#"<h2 id="#), "{html}");
    assert!(html.contains("System"), "{html}");
}

#[test]
fn nothing_is_exported_when_nothing_was_understood() {
    assert_eq!(export(""), "");
    assert_eq!(export("hello"), "");
}
