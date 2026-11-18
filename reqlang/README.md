# reqlang

The core library of the [req language](https://req-lang.org): the parser, the analysis and the formatter that
[reqtool](../reqtool) and [reqlsp](../reqlsp) are built on.

reqlang parses a specification, resolves its references, type-checks its expressions and checks its traceability. What
it finds is reported as issues, with the part of the source they are about.

If you only want to check specifications, you do not need this crate: install [reqtool](../reqtool#install), the
command-line toolkit. Use reqlang to build your own tool on top of the language.

## Install

reqlang is a Rust library. Add it to your project as a dependency of your `Cargo.toml`:

```sh
cargo add reqlang --git https://github.com/req-lang/req
```

or write it yourself:

```toml
[dependencies]
reqlang = { git = "https://github.com/req-lang/req" }
```

The crate uses the 2024 edition: you need a recent stable Rust toolchain (Rust 1.85 or later).

To work on the library itself, clone the repository and build the workspace:

```sh
git clone https://github.com/req-lang/req.git
cd req
cargo build -p reqlang
cargo test -p reqlang
```

## Example

Parse a specification, format it back, then analyse it:

```rust
use reqlang::{Specification, issue, renderer::formatter, syntax::parser};

fn main() {
    let source = std::fs::read_to_string("spec.req").unwrap();

    // Parse: the tree is incomplete when there are syntax errors.
    let (tree, info) = parser::parse(&source);
    println!("{} syntax error(s)", info.errors.len());

    // Format the tree back to source.
    let formatted = formatter::format_string(&tree, 0);
    println!("{formatted}");

    // Analyse: resolve references, check types and traceability.
    let mut spec = Specification::from(tree);
    spec.diagnose();
    let found: usize = issue::issues(&spec).into_iter().map(|issues| issues.len()).sum();
    println!("{found} issue(s)");
}
```

The source of [reqtool](../reqtool/src/main.rs) shows how a tool builds on it, for example how it reports each issue
with its position and severity.

## Legal

All the *reqlang* project is placed under the license provided in the [LICENSE](../LICENSE) file of the repository.
Any source file or code extract distributed with another project must carry the AGPL-3.0 license notice.

### Commercial Licensing

reqlang is released under the GNU Affero General Public License v3.0 (AGPL-3.0).
Under the AGPL, any software that incorporates reqlang must also be released
under the AGPL. If you wish to use reqlang in proprietary or closed-source
software, you must obtain a commercial license.

#### Who needs a commercial license

You need a commercial license if you want to:

* Incorporate reqlang into a proprietary or closed-source application
* Distribute a product built on reqlang without releasing your source code
* Use reqlang in a SaaS product without complying with AGPL network-use terms

#### Who does not need a commercial license

You do not need a commercial license if you are:

* Using reqlang for personal, academic, or research purposes
* Building open-source software released under a compatible license

#### How to obtain a commercial license

Contact: sami@qitab.io

Please describe your intended use case. Licensing terms are agreed
on a per-project basis.

© 2021-2026 Sami Dahoux. All rights reserved.
