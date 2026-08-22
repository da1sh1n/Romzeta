# CLAUDE.md — Romzeta

Repository-specific rules. The personal conventions in `~/.claude/CLAUDE.md` also apply.

## 1. Build through xtask

- **Always build binaries through `xtask`.** Shipped binaries must be signed; never produce a
  release artifact with a bare `cargo build --release`.

```
cargo run -p xtask -- release          build and sign launcher, listener, installer
cargo run -p xtask -- verify <exe>...  check against keys/romzeta.pub and keys/dev.pub
cargo run -p xtask -- sign <exe>...    sign in place
cargo run -p xtask -- keygen           dev signing key -> keys/dev.pub
cargo run -p xtask -- version          project version + every crate's
```

There is no `cargo xtask` alias — the repo has no `.cargo/config.toml`. Use the full
`cargo run -p xtask --` form.

## 2. Version bumps on "done"

- **When the user says "done", bump the version of the crate that changed.** "Done" means the
  feature or fix is finished — it is the signal to edit `version` in that crate's `Cargo.toml`,
  and nothing else. It is not permission to commit — see the git rule in the global CLAUDE.md.

| What was done | Bump | `0.6.0` becomes |
|---|---|---|
| Bug fix, typo, wording, small correction | patch (`z`) | `0.6.1` |
| New feature, behaviour change, refactor a user can notice | minor (`y`) | `0.7.0` |

- Only the crate that changed moves. A launcher feature bumps `launcher/Cargo.toml`, not
  listener or installer.
- Several crates touched in one "done"? Bump each by its own kind of change.
- Never bump the major (`x`) this way — that is `project_version` in the workspace
  `Cargo.toml`, it moves for every crate at once, and only when the user asks for it.
- Unsure which of the two it was? Ask, in one line. Do not guess.
- After bumping, say which crate went to which version.

## 3. graphify

This repo has a knowledge graph at `graphify-out/` with god nodes, community structure, and
cross-file relationships. Usage rules are in the global CLAUDE.md.
