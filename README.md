# Gitu

[![CI](https://github.com/FizzWizzleDazzle/gitu/actions/workflows/ci.yml/badge.svg)](https://github.com/FizzWizzleDazzle/gitu/actions/workflows/ci.yml)

A fast terminal UI for Git with syntax highlighting and complete workflow management.

## Installation

### npm (recommended)

```bash
npm install -g gitu-git
```

### Cargo

```bash
cargo install gitu
```

### From source

```bash
git clone https://github.com/FizzWizzleDazzle/gitu.git
cd gitu
cargo install --path .
```

### GitHub Releases

Download pre-built binaries from the [Releases page](https://github.com/FizzWizzleDazzle/gitu/releases).

## Features

**Multi-Panel Interface**
- Status: Stage/unstage files, commit, amend, discard changes, create stashes, preview diffs
- Log: Browse history with graph, search commits, navigate diffs
- Stash: Apply, pop, and drop stashes
- Branches: View, switch, create, delete, and merge branches

**Visual**
- Syntax highlighting for all file types
- File-by-file diff navigation
- Git decorations (branches, tags, HEAD)
- Split-view diff preview
- Help popup with all keybindings

**Git Operations**
- Staging, committing, amending
- Branch management and merging
- Cherry-pick, revert, checkout
- Discard file changes
- Remote operations (fetch, push, pull)
- Commit search by message or author

## Usage

```bash
cd your-git-repo
gitu
```

```
gitu --help     # Show help
gitu --version  # Show version
```

## Key Bindings

**Global**
- `1-4` Switch panels | `?` Help | `q` Quit | `Esc` Cancel
- `PgUp/PgDn` Scroll diff by 10 lines

**Status Panel**
- `Space` Stage/unstage | `Enter` Show diff | `a` Stage all | `u` Unstage all
- `c` Commit | `A` Amend last commit | `x` Discard changes | `s` Stash
- `j/k` Navigate

**Log Panel**
- `Enter` Show diff | `t` Tree view | `/` Search | `y` Copy hash
- `c` Checkout | `b` Branch | `p` Cherry-pick | `r` Revert
- `f` Fetch | `P` Push | `U` Pull | `h/l` Navigate files

**Stash Panel**
- `a` Apply | `p` Pop | `d` Drop | `j/k` Navigate

**Branches Panel**
- `Enter` Switch | `d` Delete | `n` New | `m` Merge | `j/k` Navigate

**Search**
- Type to search | `@prefix` Search by author | `Enter` Execute | `Esc` Exit

## Tech Stack

Built with Rust using Ratatui, Crossterm, Syntect, and Clap.

## License

MIT
