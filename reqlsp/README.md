# reqlsp

A language server for any LSP editor.

reqlsp is the Language Server (LSP 3.18) of the [req language](https://req-lang.org), built on `tower-lsp` and the
`reqlang` compiler crate. It brings the checks of [reqtool](../reqtool) into the editor while you type:

* diagnostics: the issues of the specification, as underlines
* hover, completion, go to definition and find references
* document symbols, to navigate the packages, parts and requirements of a file
* formatting
* semantic tokens, for the syntax coloring

Spec: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/

## Install

You rarely run reqlsp yourself: the editor starts it. It must be available in your `PATH`. The installation is the same
as the one of reqtool. Open a terminal and copy the command below.

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqlsp/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/req-lang/req/main/reqlsp/install.ps1 | iex
```

Authentication might be required to finalize the setup. reqlsp has no command-line options: it waits for an editor to
speak to it over standard input and output. To check the installation, make sure that your terminal finds it:

```sh
command -v reqlsp     # Linux / macOS
where reqlsp          # Windows
```

### Supported platforms

| System  | Architecture                    |
|---------|---------------------------------|
| Linux   | x86_64, aarch64                 |
| macOS   | Apple silicon (aarch64)         |
| Windows | x86_64                          |

On an Intel Mac, use Homebrew or build from source.

### Options of the installer

The installer always installs the latest [release](https://github.com/req-lang/req/releases): you do not need to look up
a version. It writes `reqlsp` to `/usr/local/bin` and asks for elevated privileges when that directory is not writable.
On Windows it writes `reqlsp.exe` to `%LOCALAPPDATA%\Programs\reqlsp` and adds this directory to your `PATH`: restart
your terminal afterwards.

On Linux and macOS, set `INSTALL_DIR` to install in another directory. This still installs the latest release:

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqlsp/install.sh \
  | INSTALL_DIR="$HOME/.local/bin" sh
```

Only if you need an older release, set `VERSION` to its tag in the list of releases (`<tag>` below):

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqlsp/install.sh | VERSION=<tag> sh
```

```powershell
$env:VERSION = "<tag>"
irm https://raw.githubusercontent.com/req-lang/req/main/reqlsp/install.ps1 | iex
```

### Homebrew (macOS)

```sh
brew install req-lang/tap/reqlsp
```

### From source

You need a recent stable Rust toolchain (Rust 1.85 or later).

```sh
git clone https://github.com/req-lang/req.git
cd req
cargo install --path reqlsp
```

This installs `reqlsp` in `~/.cargo/bin`.

## Use it in an editor

* **Visual Studio Code**: install the [req-vscode](https://github.com/req-lang/req-vscode) extension. It provides syntax
  coloring on its own. For diagnostics, hover and completion it uses reqlsp, which must be available in your `PATH`.
* **Any other editor**: configure it to start `reqlsp`, without arguments, as the language server of the `.req` files. It
  speaks the Language Server Protocol over standard input and output.

## License

reqlsp is released under the GNU Affero General Public License v3.0 (AGPL-3.0), see the [LICENSE](../LICENSE) file of the
repository. The [commercial licensing](../README.md#license) terms are the ones of the whole project.

## Development notes

### Compiler API used

- `reqlang::syntax::parser::parse(&str) -> (Tree, Spans, Vec<Span>)`: tree, source spans per entity/expression, syntax errors.
- `Specification::from(tree)` then `.diagnose()`: semantic analysis.
- `reqlang::issue::issues(&spec)`: `Vec<Issue>` per entity (`Reference`, `Type`, `Connection`, `Path`), with `Severity` (`Light`..`Critical`). Issues carry no span; anchor them on the entity span from `Spans.entities`.

Beware: compiler `Position` is 1-based (line, column, byte index); LSP is 0-based and UTF-16 by default.

### TODO

#### 1. Lifecycle and document sync
- [ ] Advertise `text_document_sync: TextDocumentSyncKind::FULL` in `initialize`
- [ ] Implement `did_open`, `did_change`, `did_close`
- [ ] Store documents (`DashMap<Url, Document>` or `RwLock<HashMap<..>>`); `Document` holds text, then `Tree`, `Spans`, `Specification`

#### 2. Position conversion (before diagnostics)
- [ ] `to_lsp_position(text, byte_index) -> Position` (UTF-16 units within the line)
- [ ] `to_byte_index(text, Position) -> usize` (needed for hover/definition)
- [ ] Unit tests with non-ASCII input
- [ ] Check whether `reqlang`'s `column` counts chars or bytes; prefer converting from `Position.index`
- [ ] Optional: negotiate `positionEncoding: utf-8` when the client supports it

#### 3. Diagnostics
- [ ] On open/change: parse, analyse, then `client.publish_diagnostics(uri, diags, version)`
- [ ] Syntax error `Span` -> `Diagnostic` (`ERROR`, `source: "reqlang"`)
- [ ] Semantic `Issue` -> `Diagnostic` on the entity span (`Critical`/`Severe` -> `ERROR`, `Moderate` -> `WARNING`, `Light` -> `HINT`/`INFORMATION`); message from `Display`
- [ ] Publish an empty list on close to clear stale diagnostics

#### 4. Document symbols
- [ ] `textDocument/documentSymbol`: walk the entity tree from `Tree::root()`, return nested `DocumentSymbol`s

#### 5. Hover
- [ ] Position -> byte index -> innermost entity/expression span
- [ ] Show entity path, kind, and expression type from `analysis.types`

#### 6. Go to definition / references
- [ ] `textDocument/definition` via `analysis.links.references` and `links.identifiers`
- [ ] `textDocument/references`

#### 7. Later
- [ ] Formatting via `renderer::formatter::format_string`
- [ ] Completion
- [ ] Incremental sync
- [ ] Debounce re-analysis
- [ ] Semantic tokens

#### Open questions
- [ ] Is `parse` fast enough to run on every keystroke? (see `reqlang/benches`)
- [ ] Add a unified `Diagnostic { span, severity, message }` list to `reqlang`, or build it in `reqlsp`?
