# req: The specification language

**Machine-verified, human-readable.**

req is a requirements-as-code language for spec-driven workflows: specify first, verify early.
Requirements are written in plain text files that mix plain English with formal expressions (progressive formalization).
The glossary of terms (packages, parts, attributes), the requirements and their traceability links live in the same
`.req` file, and the open-source checker `reqtool` verifies references, types and traceability.

> req is under active development: the language and its documentation are subject to change.

This is the master repository for everything that happens in req. It holds the crates below, which are the tools you
install and the libraries they are built on.

## A first look

```
package System
  part Model
    let temperature in real \ C \
  part

  package Requirements
    requirement Model_scope top bottom
      on System::Model
    is
      @@
        The {System::Model} shall keep its temperature under control.
      @@
    requirement

    requirement Maximum_temperature top bottom
      on System::Model::temperature
    is
      System::Model::temperature < false
    requirement
  package
package
```

```
$ reqtool check intro.req
System::Requirements::Maximum_temperature
17:6         expected type Number got Boolean
System::Model::temperature < false
found 1 issue(s) (67.06µs)
```

The checker points at the exact place where the specification is wrong. Try it without installing anything in the
[playground](https://req-lang.org/playground.html).

## The crates

| Crate                    | What it is                                                                         |
|--------------------------|------------------------------------------------------------------------------------|
| [reqtool](reqtool)       | The main command-line toolkit: analysis and much more.                             |
| [reqlsp](reqlsp)         | A language server for any LSP editor.                                              |
| [reqlang](reqlang)       | The core library: the parser, the analysis and the formatter that the others use.  |

## Install

reqtool is the only binary you will need to install. It is a general utility program that you use to check the validity
of a specification. Open a terminal and copy the command below.

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.ps1 | iex
```

Authentication might be required to finalize the setup. Once it is done you should be able to run `reqtool help`.

Each crate has its own instructions, including the other platforms and building from source:
[reqtool](reqtool#install), [reqlsp](reqlsp#install) and [reqlang](reqlang#install).

For an editor, use the [req-vscode](https://github.com/req-lang/req-vscode) extension for Visual Studio Code, which
relies on `reqlsp` for diagnostics, hover and completion. Any other editor that speaks the Language Server Protocol can
run `reqlsp` directly.

## Learn more

* [Getting started](https://req-lang.org/tutorials/getting-started.html): install the tools, write a first
  specification, read a diagnostic, formalize a requirement and trace it.
* [Reference](https://req-lang.org/reference.html): the detailed and formal specification of the language.
* [Why](https://req-lang.org/why.html): why req exists and what you can do with it.
* [Use req with LLMs and agents](https://req-lang.org/agents.html): an agent reads the language instead of guessing it.
  The documentation is also published as [`/llms.txt`](https://req-lang.org/llms.txt).

## Building from source

You need a recent stable Rust toolchain (the crates use the 2024 edition, so Rust 1.85 or later).

```sh
git clone https://github.com/req-lang/req.git
cd req
cargo build --release   # the binaries are target/release/reqtool and target/release/reqlsp
cargo test
```

`scripts/build.sh` cross-compiles `reqtool` and `reqlsp` for every supported platform into `release/`. It needs
`cargo-zigbuild`, `zig`, `jq` and `rustup`.

## Community

req is developed in the open. Open an issue to report a problem or suggest an improvement, or join the
[Discord server](https://discord.gg/uDZKp66Jjz) to talk with other users and with the maintainers. The
[community page](https://req-lang.org/community.html) tells more.

[Qitab](https://qitab.io) builds the commercial editor for req, for advanced analysis and visualization.

## License

req is released under the GNU Affero General Public License v3.0 (AGPL-3.0), see the [LICENSE](LICENSE) file. Any source
file or code extract distributed with another project must carry the AGPL-3.0 license notice.

If you wish to use req in proprietary or closed-source software, you must obtain a commercial license. Contact
sami@qitab.io and describe your intended use case: licensing terms are agreed on a per-project basis. The
[reqtool](reqtool#legal) and [reqlang](reqlang#legal) READMEs say who needs one.
