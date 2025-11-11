# Gitu

A fast terminal UI for Git with syntax highlighting and complete workflow management.

## Features

**Multi-Panel Interface**
- Status: Stage/unstage files, commit, create stashes, preview diffs
- Log: Browse history with graph, search commits, navigate diffs
- Stash: Apply, pop, and drop stashes
- Branches: View, switch, create, and delete branches

**Visual**
- Syntax highlighting for all file types
- File-by-file diff navigation
- Git decorations (branches, tags, HEAD)
- Split-view diff preview

**Git Operations**
- Staging, committing, stashing
- Branch management
- Cherry-pick, revert, checkout
- Remote operations (fetch, push, pull)
- Commit search by message or author

## Installation

```bash
cargo install --path .
```

## Usage

```bash
cd your-git-repo
gitu
```

## Key Bindings

**Global**
- `1-4` Switch panels | `q` Quit | `Esc` Cancel

**Status Panel**
- `Space` Stage/unstage | `Enter` Show diff | `a` Stage all | `u` Unstage all
- `c` Commit | `s` Stash | `j/k` Navigate

**Log Panel**
- `Enter` Show diff | `t` Tree view | `/` Search | `y` Copy hash
- `c` Checkout | `b` Branch | `p` Cherry-pick | `r` Revert
- `f` Fetch | `P` Push | `U` Pull | `h/l` Navigate files

**Stash Panel**
- `a` Apply | `p` Pop | `d` Drop | `j/k` Navigate

**Branches Panel**
- `Enter` Switch | `d` Delete | `n` New | `j/k` Navigate

**Search**
- Type to search | `@prefix` Search by author | `Enter` Execute | `Esc` Exit

## Tech Stack

Built with Rust using Ratatui, Crossterm, and Syntect.

## License

MIT
