# reqtool

The main command-line toolkit of the [req language](https://req-lang.org): analysis and much more.

reqtool is the only binary you will need to install. It is a general utility program that you use to check the validity
of a specification: it parses the `.req` files, resolves their references, type-checks their expressions and checks
their traceability. It also formats them.

## Install

Open a terminal and copy the command below.

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.ps1 | iex
```

Authentication might be required to finalize the setup. Once it is done you should be able to run the following command:

```sh
reqtool help
```

A help message should be displayed indicating the available commands and the usage of the program.

### Supported platforms

| System  | Architecture                    |
|---------|---------------------------------|
| Linux   | x86_64, aarch64                 |
| macOS   | Apple silicon (aarch64)         |
| Windows | x86_64                          |

On an Intel Mac, use Homebrew or build from source.

### Options of the installer

The installer always installs the latest [release](https://github.com/req-lang/req/releases): you do not need to look up
a version. It writes `reqtool` to `/usr/local/bin` and asks for elevated privileges when that directory is not writable.
On Windows it writes `reqtool.exe` to `%LOCALAPPDATA%\Programs\reqtool` and adds this directory to your `PATH`: restart
your terminal afterwards.

On Linux and macOS, set `INSTALL_DIR` to install in another directory. This still installs the latest release:

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.sh \
  | INSTALL_DIR="$HOME/.local/bin" sh
```

Only if you need an older release, set `VERSION` to its tag in the list of releases (`<tag>` below):

```sh
curl -fsSL https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.sh | VERSION=<tag> sh
```

```powershell
$env:VERSION = "<tag>"
irm https://raw.githubusercontent.com/req-lang/req/main/reqtool/install.ps1 | iex
```

### Homebrew (macOS)

```sh
brew install req-lang/tap/reqtool
```

### From source

You need a recent stable Rust toolchain (Rust 1.85 or later).

```sh
git clone https://github.com/req-lang/req.git
cd req
cargo install --path reqtool
```

This installs `reqtool` in `~/.cargo/bin`.

## Usage

```sh
reqtool check spec.req            # check a specification and report its issues
reqtool check --json spec.req     # the same report as JSON, for tools and agents
reqtool fmt spec.req              # print the formatted specification
reqtool fmt spec.req -o out.req   # write it to a file instead
```

`reqtool check` exits with the status 0 even when it reports issues. Scripts read `issue_count` in the JSON report
instead. `reqtool fmt` refuses a file with syntax errors, since formatting an incomplete tree would lose code.

Follow the [getting started tutorial](https://req-lang.org/tutorials/getting-started.html) to write and check a first
specification.

## Legal

All the *reqtool* project is placed under the license provided in the LICENSE file.
Any source file or code extract distributed with another project must carry the AGPL-3.0 license notice.

### Commercial Licensing

reqtool is released under the GNU Affero General Public License v3.0 (AGPL-3.0).
Under the AGPL, any software that incorporates reqtool must also be released
under the AGPL. If you wish to use reqtool in proprietary or closed-source
software, you must obtain a commercial license.

#### Who needs a commercial license

You need a commercial license if you want to:

* Incorporate reqtool into a proprietary or closed-source application
* Distribute a product built on reqtool without releasing your source code
* Use reqtool in a SaaS product without complying with AGPL network-use terms

#### Who does not need a commercial license

You do not need a commercial license if you are:

* Using reqtool for personal, academic, or research purposes
* Building open-source software released under a compatible license

#### How to obtain a commercial license

Contact: sami@qitab.io

Please describe your intended use case. Licensing terms are agreed
on a per-project basis.

© 2021-2026 Sami Dahoux. All rights reserved.
