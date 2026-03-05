# git-plan

Fine-grained git staging tool. Stage changes by hunk or by line number instead of whole files.

## Install

```bash
cargo install --path .
```

## Usage

### View diff

Show a formatted diff with hunk labels and continuous line numbers:

```bash
git-plan diff foo.rs
```

Example output:

```
--- a/foo.rs
+++ b/foo.rs

[a] @@ -1,5 +1,6 @@
  1 |  fn hello() {
  2 |      println!("hello");
  3 | +    println!("world");
  4 |  }
  5 |
  6 |  fn world() {

[b] @@ -8,4 +9,5 @@ fn world() {
  7 |
  8 |  fn bye() {
  9 |      println!("see ya");
 10 | +    println!("bye!");
 11 |  }
```

- Hunk labels `[a]` `[b]` ... are used with `--hunks`
- Line numbers 1-11 are continuous across hunks, used with `--lines`

### Stage changes

```bash
# By hunk
git-plan stage foo.rs --hunks a,c

# By line range
git-plan stage foo.rs --lines 3-8,15-20

# Single line
git-plan stage foo.rs --lines 3

# Entire file (same as git add)
git-plan stage foo.rs --all
```

### Interactive mode

Without `--hunks` / `--lines` / `--all`, enters interactive mode:

```bash
git-plan stage foo.rs
```

Displays the labeled diff, then prompts:

```
Stage hunks (e.g. a,b) or lines (e.g. 3-8,15):
```

Letter input (e.g. `a,b`) stages by hunk, numeric input (e.g. `3-8,15`) stages by line.

## Design

Built on `git2` (libgit2 Rust bindings), operating directly on Git objects without parsing text output.

```
Open repo via git2 -> Patch API for structured diff
-> User selects hunks or lines -> Build filtered unified diff patch
-> repo.apply(diff, ApplyLocation::Index) to stage
```

**By hunk**: Keeps selected hunks, skips others. Automatically adjusts `new_start` offsets for subsequent hunks.

**By line**: Filters lines within hunks. Unselected `+` lines are dropped, unselected `-` lines become context lines. Recalculates hunk header counts.

## License

MIT
