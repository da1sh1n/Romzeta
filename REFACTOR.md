# Romzeta — structure review and refactor

Working document for a multi-session refactor. **This file is the only thing a new session needs
to read to pick the work up.** It carries the findings, the block order, every block's spec, the
decisions already taken, the verification gate, and the progress log.

Delete it when block 21 is done.

---

## How to resume in a fresh session

1. Read this file top to bottom. Do not re-derive the findings — they are already verified
   against code.
2. Look at the **Progress log** at the bottom for the first block marked `not started`.
3. Launch **one** `sonnet` subagent for that block, using the block's spec plus the
   **Standard brief preamble** (both below).
4. When it returns, check its work against the code — not against its report — then run the
   **verification gate**.
5. Append one line to the progress log. Only then launch the next block.

Do not run two agents at once. Do not chain blocks without the user's go-ahead — the user is on
credits, not a usage limit.

---

## Status snapshot

| | |
|---|---|
| Blocks done | **10 of 23** — 1, 1b, 2, 3, 4, 5, 6, 7, 8, 9 |
| Test total | **113 passing, 0 failing** (`cargo run -p xtask -- test`) |
| Clippy | **0 errors**, 5 pre-existing warnings (assigned to blocks 12/13/14, see below) |
| fmt | **completely clean** since block 9 — the long-standing outlier sat in a body block 9 deleted |
| Git | nothing staged, nothing committed, no version bumped |
| Next block | **10 — Split `installer/src/app.rs`** |

---

## Context

The README says the tree was written fast with an LLM and needs a read-through. That pass is
underway: `*/human_check_todo.md` shows keeper 6/6, listener 10/12, launcher 4/31, installer
0/24, common / sigblock / trust / xtask 0.

**Evidence standard: code only.** The comments and `.md` files are AI output of the same vintage
as the code, so nothing here rests on a comment saying so. Where a comment was the only evidence
for something, the finding was dropped or re-derived from the code. Two comments were checked
against the code and found false — see §0.

**Verdict: the project structure is right.** Crate split, `default-members` forcing build order,
one file per job, one `constants.rs` per crate, `tests/` outside the crate under test, the
sign→embed order — all of it holds. Nothing here moves or renames a crate.

What is wrong: **the contracts between the four programs are each written down two or three
times.** Two files hold four jobs each. One failure path killed the launcher with no window, no
message and no log line (fixed, block 1). And three of the four binaries have no icon.

---

## Decisions already taken — do not re-litigate

| Decision | Settled as |
|---|---|
| Scope | Single-source the contracts **and** split the two big files |
| Shared home | **`common`** — no new crate |
| Timing | Refactor lands **before** the user's `human_check_todo.md` read-through |
| Execution | One subagent per block, strictly sequential, `sonnet` for every block |
| `gameDir` rule | The installer's stricter `parts.len() >= 3` wins |
| `romzetaDataDir()` | Returns `Option<PathBuf>`, **no fallback baked in** — callers keep their own policy |
| `Entry` + serde | Behind a `catalog` cargo feature so serde reaches only launcher and installer |
| Worktrees | **Never** — the user has files open in VS Code |
| Version bumps | Not part of any block. They happen when the user says "done" |
| Commits | Never implied by finishing. The user asks, explicitly, each time |

---

## §0 — The doc layer is unverified, and demonstrably wrong in places

~2,650 lines of prose (`structure.md` ×3, `TODO.md` ×4, `README.md`, `listener/README.md`,
`SIGNING.md`) plus heavy doc comments throughout, all written by the same process as the code.
**None of it is on any `human_check_todo.md` list** — those lists cover `src/` only. Neither is
any `tests/*.rs` file.

Two comments, checked against the code:

1. `listener/src/volume.rs:124-127` says `current_dir` is the volume root *"because the launcher
   resolves its content relative to where it runs"*. It does not:
   `launcher/src/content.rs:20-25` uses `env::current_exe().parent()`. Nothing reads the cwd.
   The `current_dir(root)` call is dead configuration.
2. `launcher/src/ui.rs:8` says *"the four IPC messages"*. The code below it handles five
   (`ui.rs:145-217`: `close`, `hide`, `launch:`, `mode:`, `order:`).

Neither is worth fixing individually. What matters for the read-through: **treat every comment
as a claim to check, not as evidence**, and delete rather than correct — CLAUDE.md §3 already
forbids most of what is there (the workspace `Cargo.toml` spends ~40 lines narrating build
order; `installer/Cargo.toml` ~35 on why `eframe` was dropped).

---

## The contract that now exists (built in block 2)

Blocks 3, 4 and 5 delete each program's copy and call these instead. This is the API as it
stands on disk today.

### `common/src/cartridge.rs`

```rust
// Names — the exe names are #[cfg(windows)] / #[cfg(not(windows))] pairs
pub const LAUNCHER_NAME: &str;      // "launcher.exe" / "launcher"
pub const KEEPER_NAME: &str;        // "keeper.exe"   / "keeper"
pub const LISTENER_NAME: &str;      // "listener.exe" / "listener"
pub const CATALOG_FILE: &str        = "catalog.json";
pub const CONFIG_FILE: &str         = "config.toml";
pub const GAMES_DIR: &str           = "games";
pub const IMAGES_DIR: &str          = "assets/images";
pub const LEGACY_IMAGES_DIR: &str   = "images";
pub const LOGS_DIR: &str            = "logs";
pub const PLAYTIME_FILE: &str       = "counter.txt";
pub const WEBVIEW_CACHE_DIR: &str   = "assets/EBWebView";

// Path helpers — all built on one private containedParts(&str) -> Option<Vec<&OsStr>>,
// which is the single path-escape security check for the whole tree.
pub fn isContained(relative: &str) -> bool;
pub fn gameDir(root: &Path, catalog_exe: &str) -> Option<PathBuf>;   // needs parts.len() >= 3
pub fn slugOf(catalog_exe: &str) -> Option<String>;                  // needs parts.len() >= 2
pub fn exeRelative(catalog_exe: &str) -> Option<PathBuf>;            // needs parts.len() >= 3
pub fn imageFile(root: &Path, catalog_image: &str) -> Option<PathBuf>;
pub fn slug(name: &str) -> String;                                   // '_' separator, "game" fallback
pub fn uniqueSlug(name: &str, taken: &mut HashSet<String>) -> String;

// The catalog row, behind the `catalog` feature
#[cfg(feature = "catalog")]
pub struct Entry { pub name: String, pub exe: String, pub image: String, pub steam: bool }

// The launcher→keeper command line. The --pid/--base/--playtime literals are PRIVATE
// module constants; both sides go through these two items or they do not compile.
pub struct KeeperArgs { pub pid: u32, pub base_dir: PathBuf, pub playtime_path: Option<PathBuf> }
impl KeeperArgs { pub fn toArgv(&self) -> Vec<OsString> }
pub fn parseKeeperArgs<I: IntoIterator<Item = OsString>>(args: I) -> Option<KeeperArgs>;
```

**The helpers take `&str`, not `&Entry`** — deliberately, so they are usable by crates that
never enable the `catalog` feature.

### `common/src/paths.rs`

```rust
pub fn romzetaDataDir() -> Option<PathBuf>;
// windows:     %LOCALAPPDATA%\Romzeta
// non-windows: $XDG_STATE_HOME/romzeta, else $HOME/.local/state/romzeta
// None when unset. var_os throughout. No fallback — each caller keeps its own.
```

### `common/Cargo.toml`

```toml
serde = { version = "1.0.229", features = ["derive"], optional = true }

[features]
catalog = ["dep:serde"]
```

**No workspace member enables `catalog` yet**, so `Entry` is not compiled by the default build.
It goes live when blocks 3 and 4 turn the feature on from `installer/Cargo.toml` and
`launcher/Cargo.toml`.

**Verified, do not re-investigate:** `cargo tree -p listener | grep serde` is non-empty, but
those are *build*-dependency edges via `winres → toml → serde`. Runtime edges
(`cargo tree --edges normal`) are 0 for listener, keeper and default `common`, and 3 with
`--features catalog`. The gating is correct.

### Behaviour change already shipped, flagged

The unified `isContained` **refuses an empty path**. The installer already did; the launcher's
old `.all()` predicate returned `true` for `""`. So a catalog entry with `"exe": ""` is now
rejected rather than joined onto the cartridge root. Intentional.

---

## Findings — every one cites code

Severity: 🔴 blocks · 🟡 should fix · 🟢 optional.

### Phase 1 — 🔴 The launcher dies silently on a hand-edited or read-only cartridge — **FIXED, block 1**

`launcher/src/catalog.rs:38,40` were:

```rust
.unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
.unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
```

`launcher/src/content.rs:41` panicked if `games/`, `logs/`, `assets/images` or
`assets/EBWebView` could not be created; `content.rs:63` panicked if a seed file could not be
written.

`launcher/src/main.rs:12` is `#![windows_subsystem = "windows"]`; the workspace release profile
is `panic = "abort"`. Stray comma in `catalog.json` → process vanished, no window, nothing in
`logs/launcher.log`. README step 2 tells users to hand-edit that file. Write-protected media →
same.

`launcher/src/config.rs:201-206` already handled the identical class of damage by falling back
to defaults. Two structurally identical files, opposite failure philosophies, and the crashing
one was the file users are told to edit.

### Phase 2 — the cross-program contracts, each defined 2-3 times

#### 2a. The cartridge on-disk layout — three definitions

| Name | Installer | Launcher | Listener / Keeper |
|---|---|---|---|
| `launcher.exe` | `constants.rs:28` `LAUNCHER_NAME` | — | listener `constants.rs:22,24` own copy + own `cfg` |
| `keeper.exe` | `constants.rs:33` `KEEPER_NAME` | `keeper.rs:22-26` inline + own `cfg` | — |
| `catalog.json` | `constants.rs:45` `CATALOG_FILE` | `catalog.rs:36`, `content.rs:57` inline | — |
| `config.toml` | `constants.rs:22` `CONFIG_FILE` | `config.rs:201`, `:316`, `content.rs:50` inline | — |
| `games/` | `constants.rs:46` `GAMES_DIR` | `catalog.rs:90`, `:92`, `content.rs:39` inline | — |
| `assets/images` | `constants.rs:54` `IMAGES_DIR` | `content.rs:39`, `assets.rs:70` inline | — |
| `logs/` | — | `content.rs:39`, `constants.rs:217` | keeper `constants.rs:23` `"logs/keeper.log"` |

🟡 A rename in one program produces a cartridge the others do not recognise, with no compile
error. The keeper hardcodes a subdirectory of a layout the launcher owns.

#### 2b. `%LOCALAPPDATA%\Romzeta` — computed three times, three fallbacks

| Purpose | Code | Fallback when the variable is missing |
|---|---|---|
| lease file dir | `common/src/lease.rs:99-120` | `temp_dir()/Romzeta` |
| listener install dir | `installer/src/listener.rs:26-28` (+ `constants.rs:129` `FOLDER`) | `None` — install refuses |
| listener log fallback | `listener/src/log.rs:67-84` | `temp_dir()` |

🟡 Three implementations of the one folder the keeper and listener must agree on — the keeper
writes the lease there (`keeper/src/run.rs:23`), the listener reads it
(`listener/src/volume.rs:158`).

#### 2c. `catalog.json` — two structs, and its path rules three times

- `installer/src/catalog.rs:21-31` `Entry` (Serialize + Deserialize) and
  `launcher/src/catalog.rs:20-30` `Game` (Deserialize) declare the same four fields
  independently. `installer/tests/catalog.rs` never imports the launcher's type, so nothing
  compiles both together.
- `gameDir()` exists twice and the two **behave differently**:
  `launcher/src/catalog.rs:85-93` takes `parts.next()? == "games"` then one more component, so
  it accepts a 2-component `games/<slug>`; `installer/src/catalog.rs:138-152` requires
  `parts.len() >= 3`. Same name, same purpose, different accept set.
- The "cannot escape the root" check, three separate implementations:
  `launcher/src/catalog.rs:68-72` (`.all()` on `Component::Normal | CurDir`),
  `installer/src/catalog.rs:141-145` (match with `_ => return None`),
  `installer/src/catalog.rs:186-193` (filter-then-check). 🟡 This is the check that stops a
  catalog entry resolving to `..\..\Windows`.
- Two slug rules: `installer/src/catalog.rs:103-114` uses `_`, falls back to `"game"`;
  `launcher/src/log.rs:43-59` uses `-`, falls back to `"game-<index>"`. A cartridge ends up with
  `games/elden_ring/` beside `logs/elden-ring/`.

#### 2d. The launcher→keeper argv contract — constants on one side, literals on the other

`keeper/src/constants.rs:15-19` defines `PID_FLAG` / `BASE_FLAG` / `PLAYTIME_FLAG`.
`launcher/src/keeper.rs:31-36` passes bare `"--pid"`, `"--base"`, `"--playtime"`. Rename a flag
in the keeper and it still compiles — `keeper/src/main.rs:42-44` returns on a missing `--pid`,
so the keeper starts, does nothing, and exits silently. 🟡

#### 2e. `Lease.cartridge_root` is written, parsed, required — and read by nobody

`common/src/lease.rs:20` declares it, `:36-37` writes it, `:52-55` parses it, and `:62` makes it
**mandatory** for `readLease` to return `Some`. Grep across the tree: no consumer reads the
field. So a keeper spawned with an empty `--base` produces a lease `readLease` rejects, and the
listener's "a game is running" gate (`listener/src/volume.rs:158`) quietly turns off. 🟡

### Phase 3 — the trust and version plumbing

- 🟡 **The `.pub` key-line parser exists three times, character-identical logic**:
  `listener/build.rs:104-109`, `installer/build.rs:296-299` (inlined into `trustAnchors`),
  `xtask/src/keys.rs:56-61`. All three are
  `.lines().map(str::trim).rfind(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))`.
  This decides what a shipped listener trusts.
- 🟡 **The `ANCHORS` codegen is emitted twice, byte-identical** — `listener/build.rs:61-71` and
  `installer/build.rs:312-322` build the same string, including the same two generated comment
  lines.
- 🟡 **The same key has two names in human-facing output.** `listener/build.rs:30` and
  `installer/build.rs:288` label it `"release"`; `xtask/src/keys.rs:36` labels it `"romzeta"`.
  So `xtask verify` prints one name and the listener's log and `--signature`
  (`listener/src/trust.rs:170-176`) print another, for the same key.
- 🟡 **`xtask sign` derives the signature role from the filename.**
  `xtask/src/main.rs:74-79`: `path.file_stem()` → `format!("romzeta-{name}")`, with
  `.unwrap_or("romzeta")`. A renamed file is silently signed with a role no verifier accepts.
- 🟡 **`xtask verify` never checks the role.** `xtask/src/sign.rs:76-93` loops the anchors and
  returns on the first key that verifies; the role is never compared, while
  `trust::attest` (`trust/src/lib.rs:112`) takes an expected role. So `xtask verify` reports a
  keeper signed as a launcher as good.
- 🟡 **Two version parsers, and the build-time one is laxer.**
  `xtask/src/manifest.rs:115-117` is `version.trim().split('.').next()?.parse()`, accepting `0`,
  `0.2`, `0.2.0-rc1`; `common/src/version.rs:37-52` rejects all three. xtask already depends on
  `common`. Today a crate versioned `"0.2"` passes `xtask release`, is signed with `"0.2"` in
  its trusted comment, and lands in the listener's `None` arm
  (`listener/src/volume.rs:115-121`) — which logs "no usable version" and **starts it anyway**.
- 🟢 `launcher/src/main.rs:28` calls `common::version::handled(env!("CARGO_PKG_VERSION"), None)`.
  Nothing in the tree invokes `launcher.exe --version`: the listener reads the version out of
  the verified signature (`listener/src/volume.rs:82-83` → `trust/src/lib.rs:129`), and the
  installer's probe is gone. It is a human affordance only.
- 🟢 The `project_version` major gate walks `workspace.members`
  (`xtask/src/manifest.rs:23-81`), which includes `testkit` (0.1.0) and `xtask` (0.2.0) —
  neither ships, neither is compared to anything at runtime.
- 🟢 `keeper.exe` is signed with role `romzeta-keeper` and attested by the installer, but
  answers neither `--version` nor `--signature` — `keeper/src/main.rs:25-50` parses only its
  three flags, and `keeper/Cargo.toml` has no `sigblock` dependency.

### Phase 4 — duplicated Win32, verified body-by-body

Three of these are **token-identical**, not merely similar:

| Code | Copy A | Copy B | Difference |
|---|---|---|---|
| `messageLoop()` | `keeper/src/window.rs:83-95` | `listener/src/trigger/windows.rs:208-221` | **none** — identical bodies |
| single-instance mutex | `launcher/src/instance.rs:29-46` | `listener/src/trigger/windows.rs:147-165` | the mutex-name constant only |
| `removeTrayIcon` | `launcher/src/tray.rs:191-199` | `listener/src/trigger/windows.rs:284-292` | **none** |
| `addTrayIcon` | `launcher/src/tray.rs:172-189` | `listener/src/trigger/windows.rs:264-281` | icon source and tip string — parameterisable |
| `setTip` | `launcher/src/tray.rs:203-211` | `listener/src/trigger/windows.rs:294-302` | launcher NUL-terminates only when truncated; listener always. Same effect (`common::utf16::wide` always appends a NUL), two spellings |
| `showTrayMenu` | `launcher/src/tray.rs:215-250` | `listener/src/trigger/windows.rs:306-340` | menu item labels and ids |

Their constants are declared separately with identical values:
`TRAY_ICON_UID = 1`, `WM_TRAYICON = WM_APP + 1`, open-item `= 1`, exit-item `= 2` —
`launcher/src/constants.rs:455-470` and `listener/src/constants.rs:40-67`.

🟡 Duplicated `unsafe` Win32 is the worst kind to keep two copies of, and two of these bodies
have already drifted apart in spelling.

### Phase 4b — 🟡 Three of the four binaries have no icon

`grep set_icon */build.rs` returns exactly one hit: `listener/build.rs:88`
(`assets/listener.ico`). Consequences, in code:

- `launcher/src/tray.rs:177` loads `LoadIconW(ptr::null_mut(), IDI_APPLICATION)` — the generic
  Windows default. The launcher's tray icon is the blank app icon, next to the listener's real
  one.
- `launcher.exe`, `keeper.exe` and `installer.exe` carry no icon resource, so Explorer, the
  taskbar and the download folder show them with the default PE icon. The installer is the one
  file a user downloads.

### Phase 5 — the two files holding four jobs each

#### 5a. `installer/src/app.rs` — 897 lines

1. Wizard navigation — `Screen`, `Mode`, `chooseVolume()` (`app.rs:425-516`)
2. Per-game editing model — `Details`, `Draft`, `Edit`, `KeeperState` (`app.rs:32-330`); never
   references `Screen`
3. Plan assembly — `plan()`, `keptEntries`, `liveEdits`, `removedEntries`, `editedGames`,
   `spaceShortfall` (`app.rs:546-722`)
4. Job orchestration — `start`, `startWrite`, `installListener`, `uninstallListener`,
   `startListenerJob`, `updateLauncher`, `updateKeeper`, `pollJob` (`app.rs:730-891`)

Import lines confirm the seams: `ui/games.rs:15` pulls only (2); `ui/mod.rs:40,151-174` calls
only (3) and (4).

Screen transitions are not centralised: `app.rs` mutates `self.screen` behind named methods at
7 sites, but `ui/mod.rs:158,231,243` writes the field bare inside drawing closures.

#### 5b. `launcher/src/ui.rs` — 436 lines

Window/webview construction (`:79-218`), the `window.__UI__` payload (`:379-436`), IPC parsing
(`:145-217` — a 70-line closure capturing eight cloned `PathBuf`s from `:116-122`), and an event
loop driven by four independent locals — `hide_deadline`, `hiding`, `exiting`, `topmost_until`
(`:222-235`) — mutated from different match arms and re-checked every pass at `:309-336`.

The IPC layer is stringly-typed on both sides with no shared definition: JS builds
`"launch:" + index` (`launch.js:104`), `"mode:" + mode` (`row.js:105`),
`"order:" + …join(",")` (`arrange.js:158`), `'close'` inline in `index.html:50`; Rust matches
literals and `strip_prefix` at `ui.rs:145-185` and falls through silently on anything else.

### Phase 6 — smaller items, all code-verified

- 🟡 `launcher/src/log.rs` never touches `logs/launcher.log` — it builds per-game
  `out.log`/`err.log` handles (`:25-38`) and a `slug()` (`:43-59`). Meanwhile `launch.rs:20` does
  `use crate::log;` and calls `common::log::appendLine` fully-qualified ten times in the same
  file.
- 🟡 `common::log::appendLine(&base.join(LOG_FILE), …)` appears **24 times** in
  `launcher/src/` (counted), re-deriving the path at every call. `listener::log::Log`
  (`listener/src/log.rs:19-56`) is the shape that solves it.
- 🟡 `installer/src/copy.rs:19-42` defines `enum Error { Cancelled, Io }`; `:38` renders
  `Cancelled` as the literal `"cancelled"`; every caller stringifies immediately
  (`cartridge.rs:206,218,225,244,254`, `app.rs:838,855`); and `ui/mod.rs:454` recovers it with
  `problem == "cancelled"`. A typed distinction destroyed and rebuilt by string compare across
  three modules.
- 🟡 `installer/src/app.rs:628,633` — two `.expect()` calls whose safety depends on a blocker
  loop 30 lines earlier at `:593-597`. Two passes joined by an assumption.
- 🟡 `launcher/src/config.rs:403` (`syncDefaults`) discards its write result with `let _ =`;
  `store()` at `:355-360` logs the same failure. Silent on read-only media.
- 🟡 `installer/src/volume.rs:42-82` and `listener/src/trust.rs:99-120` both hand-roll
  open → `sigblock::hasBlock` → read → `trust::attest`. The listener additionally holds the file
  handle open across verify-then-exec (`trust.rs:36`), a real difference worth preserving.
- 🟡 `launcher/Cargo.toml` has no `trust` dependency; `launcher/src/keeper.rs:29` spawns
  whatever `keeper.exe` sits beside it on removable media, unverified — while
  `listener/src/volume.rs:65` refuses to spawn an unverified `launcher.exe` in the same folder.
- 🟢 Five naming deviations, tree-wide. Everything else is clean: `grep -E "fn [a-z]+_[a-z]"`
  across `*/src/` returns only these plus winit trait impls; JS, CSS and the four banner tiers
  are fully conformant in every file.

  | File | Now | Should be |
  |---|---|---|
  | `installer/src/ui/mod.rs:204` | `fn back_target` | `backTarget` |
  | `installer/src/app.rs:356,359` | `staleLauncher`, `staleKeeper` fields | `stale_launcher`, `stale_keeper` |
  | `installer/src/cartridge.rs:43` | `exeRelative` field | `exe_relative` |
  | `launcher/src/tray.rs` | `iconPresent` field (`trayHwnd` went in block 9) | `icon_present` |
  | `launcher/src/ui.rs:63` | `let initScript` (shadows `fn initScript` at `:384`) | `let init_script` |

  Correctly left alone: `installer/src/shell.rs:128,176,185` (winit `ApplicationHandler` trait
  methods) and `installer/src/font.rs:85` (`cbSize`, a Win32 struct field).
- 🟢 `tests/common/mod.rs` is duplicated **seven times** (28 lines each; `diff`-identical except
  `LONGEST_TEST_NAME`) across common, installer, launcher, listener, sigblock, trust, xtask —
  and that constant must be hand-updated whenever a test is renamed.
- 🟢 `launcher/tests/version.rs:24-32` and `installer/tests/version.rs:23-31` are identical
  assertion code (only the comments differ), and both hand-roll a parser
  `common::version::parse` already provides.
- 🟢 `embedResources()` is duplicated across `launcher/build.rs:19`, `keeper/build.rs:14`,
  `installer/build.rs:176`, `listener/build.rs:87`, differing only in one string and the icon
  call. `listener/build.rs` alone spells it `embed_resources`.
- 🟢 `launcher/src/ui/theme.js:17-19` hardcodes `56`, `32` and `1000` as fallbacks for
  `DEFAULT_BORDER_GAP`, `DEFAULT_IMAGE_GAP` and `MIN_LOADING_AFTER_FAIL`
  (`launcher/src/constants.rs:104,105,200`). Rust fills every setting before serialising
  `window.__UI__` (`ui.rs:379-436`), so the fallbacks are unreachable and can only drift.
- 🟢 `.graphifyignore` excludes `**/tests.rs`; commit `30a3843` moved tests into `tests/`
  directories, so nothing matches. The graph shows the result: `runTest()` is god node #2, #4
  and #6, and three top communities are test communities. One line: `**/tests/`.
- 🟢 `installer/src/ui/` calls `ui.colored_label(WARN|BAD|GOOD, …)` 34 times,
  `RichText::new(…).strong()` 14 times and `Frame::group(ui.style())` 5 times with no helper;
  `staleLauncher()`/`staleKeeper()` (`ui/games.rs:90-135`) are near-identical bodies.
- 🟢 `launcher/src/order.rs:21-35` and `launcher/src/ui/row.js:30-41` implement the same
  normalisation in two languages; only the Rust side has tests
  (`launcher/tests/order.rs:58-64`).
- 🟢 `launcher/src/ui/backdrop.js` (558 lines) holds two renderers dispatched at `:442` that
  never run together: particles (`:241-335`), fog (`:341-440`).
- 🟢 `build.ps1:49-60` and `build.sh:40-56` each re-implement the "is there a trust anchor? if
  not, keygen" check in two languages.

### Things that look like findings and are not (checked in code)

- `installer/src/constants.rs:126` `STALE_CONFIG_FILE` has the same **value** as `CONFIG_FILE`
  four lines up, but `installer/src/listener.rs:119` joins it to the **listener's install dir**,
  while `CONFIG_FILE` names the **cartridge's** launcher config. Two different files that share
  a filename. **Do not collapse them.** Renaming it `LEGACY_LISTENER_CONFIG` is optional.
- `installer/src/steam.rs` vs `launcher/src/steam.rs` — same filename, no shared function or
  constant. The installer parses `appmanifest_*.acf` at add time; the launcher polls
  `HKCU\…\ActiveProcess` at launch time.
- `installer/src/version.rs` vs `listener/src/version.rs` — different jobs over one shared
  `common::version` (payload version vs own version). The launcher has none.
- `launcher/src/log.rs`, `listener/src/log.rs`, `common/src/log.rs` are three different jobs,
  not three copies: plumbing, a resolved-path handle, and per-game stdio.
- `launcher/src/constants.rs` at 470 lines: ~250 of them are the `SETTINGS` table that
  `Config::default` (`config.rs:132`), `load` (`:208`) and `syncDefaults` (`:378`) all walk. A
  real config surface.
- The `config.toml` **schema** is single-sourced. Only its filename and its seed file are
  duplicated.
- `common::lease` and `common::constants::LAUNCHER_INSTANCE_MUTEX` are the model to copy: one
  crate owns the format, both sides call it.
- Thin files (`keeper.rs` 53, `instance.rs` 54, `order.rs` 57) are each one cohesive job. Not
  merge candidates.

---

## How this gets executed

**One subagent at a time. Never two.** Each agent owns one block, finishes it, and reports back
before the next is launched. Nothing runs in parallel — so no agent is left half-done when
credits run out, and no two agents ever hold the same file.

The loop, per block:

1. Launch **one** agent with the block's spec, the exact file list it owns, and the files it
   must not touch.
2. It finishes. Check its work against the code — not against its own report.
3. Run the verification gate and show the user the real output.
4. Log one line in the progress table: block, files changed, verification result.
5. Only then launch the next.

If a block fails verification, it is re-run or fixed before moving on — the next block is not
started on a broken tree.

**Model: `sonnet` for every block.** The blocks are implementation against a spec that is
already written, and the design calls (which rule wins, what the signature is, where the
boundary sits) are made in the brief before the agent starts, not by the agent. Blocks 1, 1b and
2 showed the briefs carry enough for that. If a block comes back weak, re-run that one block
rather than quietly escalating every block after it.

**Paused between blocks unless told otherwise.** The user is on credits, not a usage limit, so
agents are launched one at a time on an explicit go-ahead.

**No worktrees** — the user has files open in VS Code, and a worktree agent would edit a copy
their buffers never see.

### Standard brief preamble — paste into every subagent brief

> You are editing the Romzeta Rust workspace. Read `CLAUDE.md` in the repo root and
> `~/.claude/CLAUDE.md` before you start; both bind you. The rules that trip agents up most:
>
> - **Naming overrides Rust idiom**: `camelCase` functions/methods, `snake_case`
>   variables/parameters/struct fields, `PascalCase` types, `ALL_CAPS` constants, `snake_case`
>   files. `#![allow(non_snake_case)]` lives at the crate root once — never add per-item
>   `#[allow]`. Never rename anything to silence a linter. The one exception is a name fixed by
>   an external contract (trait method signatures, serde field names bound to the on-disk
>   format, FFI symbols, Win32 struct fields).
> - **Comments**: four tiers, exactly ten `#`/`=`/`-` on each side of the title.
>   `########## ALL CAPS ##########`, `========== Title Case ==========`,
>   `---------- lowercase ----------`, then plain `//`. **One tier-1 banner per file.** Explain
>   *why*, never *what*. No file-header essays, no changelog comments, no commented-out code.
>   Keep them short.
> - **No `unwrap()` / `expect()`** outside tests and `main`-level startup — propagate.
> - **Prefer `match` over `if let … else`.**
> - **No new dependencies** without asking first.
> - **No fake work**: no stubs, no TODO placeholders, no swallowed errors, no mock data in a
>   real code path.
> - **Do not run any git write command** — no `add`, `commit`, `push`, `checkout`, `stash`,
>   `reset`, `restore`, branch operations. Read-only git is fine.
> - **Do not bump any crate version.**
> - **Do not create any file that is not in your owned-file list** — especially not a `.md`
>   summary, report, or notes file. Put findings in your reply.
> - **Edit only the files listed as yours.** If a change seems to need another file, stop and
>   say so in your reply instead.
> - Build and verify before reporting. `cargo run -p xtask -- test`,
>   `cargo clippy --workspace --all-targets`, `cargo fmt --all -- --check`. Report the real
>   output, including failures.

---

## The block order

Code first, all of it, then the docs — because every doc describes code that is about to move.

`Est.` is roughly how long one agent takes on the block, not wall-clock for the session. The
whole remaining run is about **4 working days** of agent time.

| # | Block | Est. | Owns (and only these) |
|---|---|---|---|
| 1 | Launcher fails soft | 1 h | `launcher/src/{catalog,content,main}.rs`, new `launcher/tests/catalog_damage.rs` |
| 1b | `common::reg` raw-pointer lint | 45 min | `common/src/reg.rs`, `launcher/src/steam.rs`, `installer/src/{autoplay,font,listener}.rs` |
| 2 | Create the contract | 3 h | new `common/src/{cartridge,paths}.rs`, `common/src/lib.rs`, `common/Cargo.toml`, new `common/tests/cartridge.rs` |
| 3 | Installer adopts it | 2 h | `installer/src/{constants,catalog,listener,cartridge}.rs`, `installer/Cargo.toml`, `installer/tests/catalog.rs` |
| 4 | Launcher adopts it | 2 h | `launcher/src/{catalog,config,content,keeper,log,assets,constants}.rs`, `launcher/Cargo.toml` |
| 5 | Listener + keeper adopt it | 1.5 h | `listener/src/{constants,log,trust}.rs`; `keeper/src/{constants,main,run}.rs`; `common/src/lease.rs` |
| 6 | One catalog contract test | 45 min | new `common/tests/catalog.rs` (+ its `tests/common/mod.rs`) |
| 7 | Trust + version single-sourced | half a day | `trust/src/lib.rs`, new `trust/src/keyfile.rs`; `listener/build.rs`, `installer/build.rs`; `xtask/src/{keys,sign,main,manifest,release}.rs` |
| 8 | Create `common::win32` | 2 h | new `common/src/win32.rs`, `common/src/lib.rs`, `common/Cargo.toml` |
| 9 | Three crates adopt win32 | 2 h | `launcher/src/{tray,instance,constants}.rs`; `listener/src/trigger/windows.rs`, `listener/src/constants.rs`; `keeper/src/window.rs` |
| 10 | Split `installer/src/app.rs` | half a day | `installer/src/app.rs` → new `game.rs`, `jobs.rs`, into `cartridge.rs`; `lib.rs`, `ui/mod.rs`, `ui/games.rs` |
| 11 | Split `launcher/src/ui.rs` | 3 h | `launcher/src/ui.rs` → new `ipc.rs`, `page.rs`; `lib.rs` |
| 12 | Launcher sweep | 2 h | `launcher/src/log.rs`→`game_output.rs`, `config.rs`, `ui.rs`, `tray.rs`, `launch.rs`, `steam.rs`, `catalog.rs`, `ui/theme.js` |
| 13 | Installer sweep | 3 h | `installer/src/{copy,cartridge,app,work}.rs`, `installer/src/ui/{mod,games,listener}.rs` |
| 14 | Tooling sweep | 2 h | 4× `build.rs`, `testkit/src/*`, 7× `tests/common/mod.rs`, 2× `tests/version.rs`, `.graphifyignore`, `build.ps1`, `build.sh` |
| 14b | `catalog.json` gets one parser | 2 h | new `common/src/catalog.rs`, `common/src/lib.rs`; `installer/src/catalog.rs`, `launcher/src/catalog.rs`, `common/tests/catalog.rs` |
| 15 | Icons — **BLOCKED** | 30 min | `launcher/build.rs`, `keeper/build.rs`, `installer/build.rs`, `launcher/src/tray.rs` |
| 16 | `README.md` | 1 h | that file |
| 17 | `launcher/structure.md` | 1 h | that file |
| 18 | `installer/structure.md` | 1 h | that file |
| 19 | `listener/structure.md` | 45 min | that file |
| 20 | `SIGNING.md` + `listener/README.md` | 1 h | those two |
| 21 | The checklists | 1 h | 4× `TODO.md`, 8× `human_check_todo.md` |

Original phase estimates these came from: stop the silent deaths ~1 hour · one home for the
cartridge contract ~1 day · one trust and version story ~half a day · one Win32 scaffold ~half a
day · split the two big files ~1 day · the mechanical sweep ~half a day. The doc blocks had no
estimate in the original plan; those are new.

**Block 15 is blocked on the user**: it needs `launcher.ico`, `keeper.ico` and `installer.ico`,
which do not exist in the tree. Run it last of the code blocks, or skip it and say so.

**Blocks 16-21 rewrite every doc from the code as it then stands** — not from the current prose,
which §0 shows is unverified and wrong in at least two places. Each agent is told to read the
source and describe what it finds, and to delete rather than carry forward anything it cannot
confirm in the code. CLAUDE.md §3 applies: a banner is a label, not an essay; no narration of
why an approach beat an alternative.

---

## The blocks — specs

### Block 1 — stop the silent deaths ✅ done

- `launcher/src/catalog.rs:35-59` — return `Vec::new()` on read/parse failure and log the reason
  via `common::log::appendLine`, matching `config::load`'s behaviour at `config.rs:201-206`.
- `launcher/src/content.rs:28-66` — `ensureLayout` returns `Result<(), String>`, attempting all
  six steps regardless of an earlier failure and returning the first problem; `main.rs` logs and
  continues with whatever folders exist.
- New `launcher/tests/catalog_damage.rs`: malformed JSON, missing file, and a good file.

`resolveBaseDir()` keeps its two `.expect()` calls — legitimate startup preconditions.

### Block 1b — `common::reg` raw-pointer lint ✅ done

`clippy::not_unsafe_ptr_arg_deref` (deny-by-default) fired 3× in `common/src/reg.rs:47,58,210`,
which killed `common` and therefore every crate under clippy. Pre-existing, last touched in
`f23f940`. Fixed with a `Root` enum rather than marking the functions `unsafe`:

```rust
pub enum Root { CurrentUser, LocalMachine }
impl Root { fn toHkey(self) -> HKEY }
pub fn open(root: Root, path: &str, access: u32) -> Option<Key>
pub fn create(root: Root, path: &str) -> Result<Key, String>
pub fn deleteKey(root: Root, path: &str)
```

`HKEY_CURRENT_USER` / `HKEY_LOCAL_MACHINE` re-exports were dropped. Call sites use alias
imports: `use common::reg::{self, Root::CurrentUser as HKCU, Root::LocalMachine as HKLM};`

### Block 2 — one home for the cartridge contract ✅ done

Built `common/src/cartridge.rs` and `common/src/paths.rs` — full API recorded above under
**The contract that now exists**. Also `common/tests/cartridge.rs`: 4 tests / 27 checks.

Still to do inside later blocks: settle `Lease.cartridge_root` (block 5) and point
`launcher/src/log.rs`'s slug at `cartridge::slug` so `logs/<slug>/` matches `games/<slug>/`
(block 4 or 12).

### Block 3 — installer adopts the contract ✅ done

Owns: `installer/src/constants.rs`, `installer/src/catalog.rs`, `installer/src/listener.rs`,
`installer/src/cartridge.rs`, `installer/Cargo.toml`, `installer/tests/catalog.rs`.

Also allowed to touch call sites in `installer/src/{app,detect}.rs` and
`installer/src/ui/{mod,games}.rs` **only** to update import paths and argument types — no logic
changes there.

1. `installer/Cargo.toml`: enable the feature —
   `common = { path = "../common", features = ["catalog"] }`.
2. `installer/src/constants.rs`: delete `CONFIG_FILE` (`:22`), `LAUNCHER_NAME` (`:28`),
   `KEEPER_NAME` (`:33`), `CATALOG_FILE` (`:45`), `GAMES_DIR` (`:46`), `IMAGES_DIR` (`:54`).
   Re-point every user at `common::cartridge::*`.
   **`STALE_CONFIG_FILE` (`:126`) stays** — same value, different file (see "Things that look
   like findings and are not"). `FOLDER` (`:129`) is folded into step 4.
3. `installer/src/catalog.rs`: delete `Entry`, `isFalse`, `slug`, `uniqueSlug`, `gameDir`,
   `slugOf`, `exeRelative`, `imageFile`. Re-export or import `common::cartridge::Entry`. Keep
   `read`, `write`, `exePath`, `imagePath`, `toRelativeString` — those are installer-only jobs
   built on the shared constants.
   **Signature change**: the shared helpers take `&str`, not `&Entry`. Call sites become
   `cartridge::gameDir(root, &entry.exe)`, `cartridge::slugOf(&entry.exe)`,
   `cartridge::exeRelative(&entry.exe)`, `cartridge::imageFile(root, &entry.image)`.
4. `installer/src/listener.rs:26-28`: replace the hand-rolled `%LOCALAPPDATA%\Romzeta` with
   `common::paths::romzetaDataDir()`, preserving the existing "refuse the install when it is
   `None`" policy.
5. `installer/src/cartridge.rs`: switch to the shared constants; leave `Plan`, `PlannedGame`,
   `EditedGame` alone (block 10 moves things *into* this file, not out).
6. `installer/tests/catalog.rs` imports `installer::catalog::slug` / `gameDir` and **will not
   compile** once they move — re-point it at `common::cartridge`. `installer/tests/edit.rs:20`
   imports `installer::catalog::Entry`, so keep that path working via a `pub use`.

Known call sites to fix (from grep):
`app.rs:273,274,275,298,301,457,531,622,687,691`; `cartridge.rs:58,59,222,267,350,352,358`;
`detect.rs:35,241`; `ui/games.rs:371`; `ui/mod.rs:366`.

### Block 4 — launcher adopts the contract ✅ done

The dead `index` parameter that fed the old `game-<index>` slug fallback was removed from
`log::gameOutput`, `launch::spawn` and `launch::run`, and from `ui.rs`'s call.

Owns: `launcher/src/{catalog,config,content,keeper,log,assets,constants}.rs`,
`launcher/Cargo.toml`.

- Enable `common`'s `catalog` feature; replace the launcher's `Game` struct with
  `common::cartridge::Entry`, or keep `Game` as a type alias if the field access reads better.
- Delete the launcher's inline `gameDir` (`catalog.rs:85-93`) and `isContained`
  (`catalog.rs:68-72`) — the shared `gameDir` is stricter, which is the settled decision.
- Delete the inline `cfg` block naming `keeper.exe` (`keeper.rs:22-26`); use
  `cartridge::KEEPER_NAME`.
- `keeper.rs:31-36`: build `cartridge::KeeperArgs { … }` and spawn with `toArgv()` instead of
  the three bare string literals.
- Replace inline `"catalog.json"`, `"config.toml"`, `"games"`, `"assets/images"`, `"logs"`
  literals in `catalog.rs`, `config.rs`, `content.rs`, `assets.rs`, `constants.rs`.
- Point `log.rs`'s `slug()` at `cartridge::slug` so `logs/<slug>/` matches `games/<slug>/`.
  **This is a visible behaviour change** — existing cartridges get `logs/elden_ring/` where they
  had `logs/elden-ring/`. Flag it; do not silently migrate.

### Block 5 — listener + keeper adopt the contract ✅ done

`Lease.cartridge_root` was **dropped**, not made `Option`. Nothing read it, and the parked 2.x
saves plan carries the cartridge root in the `WM_COPYDATA` payload instead
(`listener/TODO.md` §1). That also closes the gate-disabling bug: an empty `--base` used to
produce a lease `readLease` rejected.

The `PLAYTIME_FILE` item landed in `launcher/src/ui.rs` — the only place that built
`counter.txt`, missed by block 4.

Owns: `listener/src/{constants,log,trust}.rs`; `keeper/src/{constants,main,run}.rs`;
`common/src/lease.rs`.

- `listener/src/constants.rs:22,24`: delete the `LAUNCHER_NAME` `cfg` pair; use
  `cartridge::LAUNCHER_NAME`.
- `listener/src/log.rs:67-84`: use `common::paths::romzetaDataDir()`, keeping the
  `temp_dir()` fallback policy.
- `common/src/lease.rs:99-120`: `baseDir` calls `paths::romzetaDataDir()`, keeping the
  `temp_dir()/Romzeta` fallback.
- `common/src/lease.rs`: settle `cartridge_root` — drop it, **unless** `listener/TODO.md`'s 2.x
  saves plan needs it, in which case make it `Option` so its absence stops disabling the whole
  gate. Check that TODO before deciding.
- `keeper/src/constants.rs:15-19`: delete `PID_FLAG` / `BASE_FLAG` / `PLAYTIME_FLAG`;
  `keeper/src/main.rs:25-50` parses with `cartridge::parseKeeperArgs(env::args_os().skip(1))`.
- `keeper/src/constants.rs:23`: `"logs/keeper.log"` is built from `cartridge::LOGS_DIR`.
- `keeper/src/run.rs`: playtime file name comes from `cartridge::PLAYTIME_FILE`.

Neither the listener nor the keeper enables the `catalog` feature — they must stay serde-free.
Verify with `cargo tree -p listener --edges normal | grep serde` (expect nothing).

### Block 6 — one catalog contract test ✅ done, narrowed

**The original spec is not buildable on MSVC.** It asked for one test linking both the
installer's write path and the launcher's read path. `installer/build.rs` and `launcher/build.rs`
both call `winres::WindowsResource::compile()`, which unconditionally prints
`cargo:rustc-link-lib=dylib=resource` — a directive that applies to **every** target of the
crate, tests included, not just its `[[bin]]`. A test binary linking both lib targets therefore
carries two VERSION resources and dies at link time:

```
CVTRES : fatal error CVT1100: duplicate resource.  type:VERSION, name:1, language:0x0409
LINK : fatal error LNK1123: failure during conversion to COFF: file invalid or corrupt
```

Reproduced from clean; `cargo build --workspace` (no test targets) is unaffected, so nothing
shipped is broken. `winres` 0.1.12 exposes no way to scope the directive to bins — the fix is
`cargo:rustc-link-arg-bins=<out>/resource.lib`, which means replacing `compile()` with
`write_resource_file()` plus a hand-rolled `rc.exe` invocation, or moving to the maintained
`winresource` fork. **Assigned to block 14**, which already owns the four build scripts.

What shipped instead — `common/tests/catalog.rs`, linking the installer only, 3 tests / 30
checks:

| Test | Pins |
|---|---|
| `the_installer_writes_a_catalog_it_reads_back_unchanged` | `catalog::write` → `catalog::read` field-by-field over a 3-row fixture |
| `steam_is_written_only_when_true_and_absent_means_false` | the `skip_serializing_if` / `default` pair on `Entry::steam`, read off the raw JSON |
| `every_row_the_installer_writes_survives_the_launchers_filter` | `isContained` on every written `exe` and `image` — the predicate `launcher::catalog::load` silently drops rows by — plus the two refusals that stop it passing vacuously |

Each was proved to fail by breaking a fixture field and watching the report name the row.

**Still owed**: the launcher's `load` against the installer's `write`. Settled as **block 14b** —
the parser moves into `common` so one crate holds both halves, rather than the build scripts
being changed so one test binary can hold both crates.

### Block 7 — one trust and version story ✅ done

Landed as specced. Three notes for later blocks:

- `xtask/Cargo.toml` gained `trust = { path = "../trust" }` — unavoidable, since items 4 and 6
  have `xtask` calling `trust::constants::roleForStem` and `trust::comment::build`.
- `xtask/tests/manifest.rs` lost `reads_the_leading_number`, which tested the deleted
  `manifest::major`. Replaced with two fixture-workspace tests —
  `an_unparsable_version_in_a_shipped_crate_fails` (the regression test for item 7) and
  `a_helper_crate_is_not_held_to_the_project_major` (item 8) — both proved to fail when broken.
  `common::version::parse`'s own strictness was already covered by
  `listener/tests/version.rs:44`.
- `keys::base64Line` survives as a thin wrapper over `trust::keyfile::keyLine`, because
  `xtask`'s tests and call sites already reach for that name.

- Move the `.pub` key-line parser and the `ANCHORS` codegen into `trust`, as a new
  `trust/src/keyfile.rs`, usable as a `[build-dependencies]` path dep (`installer/build.rs`
  already does this). `listener/build.rs`, `installer/build.rs` and `xtask/src/keys.rs` all
  call it.
- The anchor label is **`"romzeta"`**, settled. Not taste: `xtask/src/sign.rs:80` already builds
  its error as `format!("keys/{}.pub", anchor.name)`, which is **wrong today** for the two
  build-script sites — there is no `keys/release.pub`, the file is `keys/romzeta.pub`. Making the
  anchor name equal the `.pub` stem fixes that message by construction and kills the third
  parallel list, so `("romzeta", "romzeta.pub")` / `("dev", "dev.pub")` lives once in
  `trust/src/keyfile.rs`. Visible consequence: `listener --signature` and the listener's log
  print `romzeta` where they printed `release`.
- Add `trust::comment::build(role, version)` to pair with the existing `splitComment`
  (`trust/src/lib.rs:153`); `xtask/src/release.rs:60,66,72,86` calls it instead of format
  strings. **Two fields, not three** — `splitComment`'s doc mentions an optional trailing date
  and `xtask` does not write one, so do not invent a date argument.
- `xtask/src/main.rs:74-79` — look the role up against `trust`'s four role constants, which
  already exist in `trust/src/constants.rs` (`LAUNCHER_ROLE` … `KEEPER_ROLE`). Add a
  stem → role lookup returning `Option`, and error on an unrecognised stem naming the four it
  accepts. Deleting the `.unwrap_or("romzeta")` fallback is the point.
- `xtask/src/sign.rs:76-93` — pass the expected role through so `xtask verify` reaches the
  listener's verdict.
- `xtask/src/manifest.rs:115-117` — delete `major()`, use `common::version::parse`.
- `xtask/src/sign.rs:76-93` — preserve `verifyAll`'s shape while threading the role through: it
  deliberately reports **every** path before the first failure decides the exit code, because
  "which of these four is the bad one" is the question being asked. Not an early return.
- Limit the `project_version` gate to the four binary crates — `launcher`, `listener`, `keeper`,
  `installer`. `xtask/src/release.rs` already names those four inline; put the list in
  `xtask/src/constants.rs` once and have `manifest.rs` and `release.rs` both read it. Crates
  outside the list are still read (that walk is how their versions are known) but are not held
  to the project major.
- `listener/Cargo.toml` has **no `[build-dependencies]` section at all** — its `winres` sits
  under `[target.'cfg(windows)'.build-dependencies]`. Add a plain one for `trust`.
  `installer/Cargo.toml` already has `trust = { path = "../trust" }` there.

### Block 8 — create `common::win32` ✅ done

`common/src/win32.rs`, whole file behind `#![cfg(windows)]` and gated again on its `pub mod` line
— the same double gate `reg.rs` uses. Nothing calls it yet; block 9 does the adoption.

```rust
pub fn hiddenWindow(class: &str, title: &str, wndproc: WNDPROC) -> Option<HWND>;
pub fn messageLoop();

pub struct InstanceGuard(HANDLE);            // Drop closes a non-null handle
pub fn singleInstance(name: &str) -> Option<InstanceGuard>;

pub const TRAY_ICON_UID: u32 = 1;
pub const WM_TRAYICON: u32 = WM_APP + 1;
pub enum TrayIcon { System, Resource(*const u16) }

#[derive(Clone, Copy)]
pub struct Tray { hwnd: HWND }               // field private
impl Tray {
    pub fn new(hwnd: HWND) -> Tray;
    pub fn add(&self, icon: TrayIcon, tip: &str) -> bool;
    pub fn remove(&self);
    pub fn showMenu(&self, items: &[&str]) -> Option<usize>;
}
```

Three calls settled in the brief, all of them shaping block 9:

1. **`showMenu` takes labels and returns a zero-based index**, not caller-supplied ids —
   `AppendMenuW` gets `index + 1` internally because `TrackPopupMenu` uses `0` for "nothing
   picked". This is what lets block 9 delete `ID_MENU_OPEN`, `ID_MENU_OPEN_LOG` and
   `ID_MENU_EXIT` from both `constants.rs` files.
2. **`setTip` is private**, used only by `Tray::add`. Nothing in the tree changes a tip after the
   icon is added, so a public one would be API with no caller. Kept the listener's unconditional
   `field[n - 1] = 0;` spelling.
3. **`hiddenWindow` checks `RegisterClassW == 0`.** The keeper's copy did not — adopting the
   checked version is a small behaviour improvement that lands in block 9.

`singleInstance` keeps the "never let the guard be a reason not to run" policy: a null handle from
`CreateMutexW` still returns `Some`; only `ERROR_ALREADY_EXISTS` yields `None`.

`common` already enabled `Win32_Foundation`, `Win32_Security`, `Win32_System_Registry`,
`Win32_System_Threading` and `Win32_UI_Shell`; this added `Win32_UI_WindowsAndMessaging`,
`Win32_Graphics_Gdi` and `Win32_System_LibraryLoader` — all already enabled in the crates that
stop enabling them in block 9. LTO drops what a binary does not call, so the listener's size is
unaffected.

`clippy::not_unsafe_ptr_arg_deref` does **not** fire on `Tray`'s `HWND` methods or on
`TrayIcon::Resource(*const u16)` — an FFI hand-off is not a deref. No `#[allow]` was needed.

### Block 9 — three crates adopt win32 ✅ done

Every duplicated body from Phase 4 is gone. `listener/src/trigger/windows.rs` lost 214 lines;
`keeper/src/window.rs` lost 59; `launcher/src/instance.rs`'s whole `#[cfg(windows)]` half became
a two-line delegation carrying only the launcher's mutex-name policy.

The menu ids are now indices beside their labels, one pair per crate, so the order and the
constants cannot drift:

```rust
pub const MENU_ITEMS: [&str; 2] = ["Open Romzeta", "Exit"];   // "Open log" in the listener
pub const MENU_OPEN: usize = 0;
pub const MENU_EXIT: usize = 1;
```

`TRAY_ICON_UID` and `WM_TRAYICON` were deleted from both `constants.rs` files and now come from
`common::win32`. `WINDOW_CLASS` stays in each — its value is per-crate. So does the listener's
`TRAY_ICON_RESOURCE` with its `without_provenance` explanation.

Three `windows-sys` features came out, each grep-proven unused: `Win32_System_LibraryLoader` from
all three crates (`GetModuleHandleW` had no caller left), and `Win32_Graphics_Gdi` from the keeper
and listener. **The launcher keeps `Win32_Graphics_Gdi`** — `launcher/src/window.rs` still uses it
for the Windows-10 corner-rounding fallback.

`launcher/src/tray.rs`'s `trayHwnd: HWND` field became `tray: Tray`, so that half of block 12's
naming fix no longer exists; `iconPresent` is untouched and still block 12's.

**`cargo fmt --all -- --check` is completely clean for the first time in this refactor** — the
outlier at `listener/src/trigger/windows.rs:312` was inside `showTrayMenu`, which this block
deleted. The block order's claim that block 9 also owned two clippy warnings was wrong: the
assignment table puts all five in blocks 12, 13 and 14, and none is in a file block 9 owns.

### Block 10 — split `installer/src/app.rs` (897 lines) ⬅ NEXT

| New file | Moves | Its one job |
|---|---|---|
| `installer/src/game.rs` | `Details`, `Draft`, `Edit`, `KeeperState` + impls (`app.rs:32-330`) | one game's editable fields and whether they are ready |
| into `installer/src/cartridge.rs` | `plan()`, `keptEntries`, `liveEdits`, `removedEntries`, `editedGames`, `spaceShortfall` (`app.rs:546-722`) | derive a `Plan` — that file already owns `Plan`, `PlannedGame`, `EditedGame` |
| `installer/src/jobs.rs` | `start`, `startWrite`, `installListener`, `uninstallListener`, `startListenerJob`, `updateLauncher`, `updateKeeper`, `pollJob` (`app.rs:730-891`) | turn a user action into a `work::Job` and read it back |
| `installer/src/app.rs` | the remainder (~250 lines) | wizard state and the create-vs-edit routing rule |

Add `App::goto(Screen)` and route `ui/mod.rs:158,231,243` through it. Move `back_target`
(`ui/mod.rs:204`) beside `Screen` and rename it `backTarget`.

### Block 11 — split `launcher/src/ui.rs` (436 lines)

| New file | Moves | Its one job |
|---|---|---|
| `launcher/src/ipc.rs` | `ui.rs:145-217` as `fn handleIpc(body) -> Action`, plus an `Action` enum naming all five messages | the page's half of the wire, testable without a window |
| `launcher/src/page.rs` | `initScript` and the payload (`ui.rs:379-436`) | what the page is handed at load |
| `launcher/src/ui.rs` | window/webview + loop (~200 lines), with the four locals collapsed into one state struct | build the window and run it |

### Block 12 — launcher sweep

- Rename `launcher/src/log.rs` → `game_output.rs`; give the launcher a `Log` handle like
  `listener::log::Log`, collapsing the 24 `appendLine(&base.join(LOG_FILE), …)` call sites.
- `launcher/src/config.rs:403` logs its write failure like `store()` does.
- Fix the naming deviations in `tray.rs` (`iconPresent`) and `ui.rs:63`.
- Delete the three unreachable JS fallback literals in `ui/theme.js:17-19`.
- Fixes the two pre-existing `collapsible_if` clippy warnings at `launcher/src/ui.rs:271,272`
  (line numbers will have moved after block 11).

### Block 13 — installer sweep

- Thread a real error enum through `cartridge::apply` → `Job::outcome` → `App::outcome` so
  `installer/src/ui/mod.rs:454` stops comparing against the string `"cancelled"`.
- Fold `installer/src/app.rs:593-633`'s two passes into one loop returning `Result` per draft,
  removing both `.expect()`s.
- Extract `trust::verifyFile(path, anchors, role)` for `installer/src/volume.rs:42-82` and
  `listener/src/trust.rs:99-120`, with a `verifyAndHold` variant keeping the listener's lock.
- Fix the naming deviations in `ui/mod.rs:204`, `app.rs:356,359`, `cartridge.rs:43`.
- Add `warn`/`bad`/`ok`/`heading`/`card` helpers to `installer/src/ui/`; collapse
  `staleLauncher`/`staleKeeper` (`ui/games.rs:90-135`) into one `staleBanner`.
- Fixes the pre-existing `new_without_default` clippy warning at `installer/src/app.rs:382`.

### Block 14 — tooling sweep

- `testkit::harness!("name")` macro for the seven `tests/common/mod.rs` copies — or better, have
  `testkit::report` compute the column width from the rows it printed and delete
  `LONGEST_TEST_NAME` entirely.
- Replace both hand-rolled version tests (`launcher/tests/version.rs:24-32`,
  `installer/tests/version.rs:23-31`) with `common::version::parse`.
- One shared `embedResources` for the four build scripts; `listener/build.rs` renames its
  `embed_resources` to match. **Keep `winres::compile()` as it is** — see the constraint below.
- `.graphifyignore`: `**/tests.rs` → `**/tests/`, then `graphify update .`.
- Move `build.ps1`/`build.sh`'s keygen check into `xtask release`; both become a one-line
  delegation.
- Fixes the pre-existing `useless_format` warning at `listener/src/trust.rs:96` and
  `assign_op_pattern` at `keeper/src/run.rs:49`.

### Block 14b — `catalog.json` gets one parser

Runs straight after block 14. A separate block rather than part of it because it touches no
build script, and two blocks must never hold the same file.

**Why.** After blocks 3 and 4 both programs share `Entry` and `CATALOG_FILE`, so the *type* and
the *filename* can no longer drift. What is still written twice is the **parse and serialize**:
`installer/src/catalog.rs:29-48` and `launcher/src/catalog.rs:32-53` each call `serde_json`
against the same file. Block 6 tried to pin the two together with a test and could not — the
build scripts forbid one binary linking both crates (constraint below). Moving the parser into
`common` closes the gap at the source instead: one reader, one writer, and a round-trip test
that needs no cross-crate link at all.

**What moves.** New `common/src/catalog.rs`, behind the existing `catalog` feature:

```rust
pub fn read(root: &Path) -> Result<Vec<Entry>, String>;   // Ok(vec![]) when absent or blank
pub fn write(root: &Path, rows: &[Entry]) -> io::Result<()>;
```

`read` takes the installer's current policy verbatim (`catalog.rs:25-28`): a **missing** file is
an empty list, because a volume can carry a `.cartridge` marker and no catalog yet; a file that
is *there* but unparsable is an **error**, because overwriting it would throw away a list we
could not read. `write` keeps the pretty-print and the trailing newline — that file is
hand-edited on a finished cartridge.

**What stays put — this is the part to get right.** Only the parser moves. Each crate keeps its
own policy, because they genuinely differ:

| Stays in | Keeps | Because |
|---|---|---|
| `launcher/src/catalog.rs` | fail-soft: log the reason, return `Vec::new()` | block 1 — the README tells people to hand-edit this file, and a stray comma must cost the covers, not the window |
| `launcher/src/catalog.rs` | the `isContained` filter and its `REFUSED` log line | the launcher drops rows the installer never would; that is a launcher rule |
| `launcher/src/catalog.rs` | `payload()` | it is the page's shape, not the file's |
| `installer/src/catalog.rs` | surfacing `read`'s `Err` to the wizard | the installer must refuse to overwrite what it could not read |
| `installer/src/catalog.rs` | `exePath`, `imagePath`, `toRelativeString` | they build catalog strings, they do not parse the file |

So `launcher::catalog::load` becomes: call `common::catalog::read`, log-and-empty on `Err`, then
the existing filter. `installer::catalog::{read, write}` become one-line delegations, or go
away with their callers re-pointed — whichever reads better once the bodies are gone.

**Then finish block 6.** With both halves in `common`, add the round trip to
`common/tests/catalog.rs`: write a fixture, read it back, and assert the launcher's filter keeps
every row. Drop the `installer` dev-dependency from `common/Cargo.toml` if nothing else needs
it, and delete the comment in `common/tests/catalog.rs` explaining why the launcher is not
linked.

**Verify** that `cargo tree -p listener --edges normal | grep serde` is still empty — the new
module is behind the `catalog` feature and the listener must not gain it.

#### The constraint this works around, left in place deliberately

`installer/build.rs`, `launcher/build.rs`, `keeper/build.rs` and `listener/build.rs` all call
`winres::WindowsResource::compile()`, which prints `cargo:rustc-link-lib=dylib=resource`. That
directive applies to **every** target of the crate, not just its `[[bin]]`, so any test binary
linking two of these crates carries two VERSION resources and dies at link time:

```
CVTRES : fatal error CVT1100: duplicate resource.  type:VERSION, name:1, language:0x0409
LINK : fatal error LNK1123: failure during conversion to COFF: file invalid or corrupt
```

Nothing shipped is affected — a bin only ever carries one. `winres` 0.1.12 offers no way to
scope the directive; fixing it means bypassing `compile()`, locating `rc.exe` independently and
emitting `cargo:rustc-link-arg-bins` instead. **Decided against**: it buys one test what a
smaller refactor buys properly, and it puts SDK discovery in the build path of all four
binaries. The cost of leaving it is that no future test can link two of these four crates —
if one ever needs to, this is why it will not compile.

### Block 15 — icons — BLOCKED

Add `res.set_icon(...)` to `launcher/build.rs`, `keeper/build.rs` and `installer/build.rs`, and
switch `launcher/src/tray.rs:177` from `IDI_APPLICATION` to the launcher's own resource, the way
`listener/src/trigger/windows.rs:267` already does.

**Needs `launcher.ico`, `keeper.ico` and `installer.ico`, which do not exist in the tree.** The
user has to supply them.

### Blocks 16-21 — docs and checklists

One agent per file. Each reads the source as it then stands and writes what it finds. Anything
it cannot confirm in the code is deleted, not carried forward. Block 21 must also **add the
`tests/*.rs` files and the `.md` files to the `human_check_todo.md` lists** — no list currently
covers them (§0).

---

## Flagged, deliberately not in this plan

**The launcher spawns `keeper.exe` unverified** (`launcher/Cargo.toml` has no `trust` dep;
`launcher/src/keeper.rs:29`). The one program running off removable media verifies nothing,
while the listener refuses an unverified launcher in the same folder. Closing this means giving
the launcher `trust` plus baked-in anchors — a behaviour change, outside the agreed scope.
Block 7's shared anchor codegen is built so this is a small follow-up.

Also noted only: splitting `backdrop.js`, a JS-side parity test for `order::normalize`, and
giving `keeper.exe` `--version` / `--signature`.

---

## Verification

Nothing here changes what any program does, so the bar is "identical behaviour, smaller tree".

**Per block, before the next agent is launched** — all three, run by the main session, not taken
on an agent's word:

```
cargo run -p xtask -- test              # must stay at the running total, 0 failed
cargo clippy --workspace --all-targets  # must stay at 0 errors
cargo fmt --all -- --check              # must not add a new unformatted file
```

Green, with the real output shown to the user, or the block is fixed before anything else
starts.

`fmt` was added to the gate after block 2: block 1 left two files unformatted and neither of the
other two commands noticed.

**Known pre-existing issues, each assigned to a block — do not fix early:**

| Issue | Where | Owned by |
|---|---|---|
| `assign_op_pattern` | `keeper/src/run.rs:49` | block 14 |
| `useless_format` | `listener/src/trust.rs:96` | block 14 |
| `collapsible_if` ×2 | `launcher/src/ui.rs:271,272` | block 12 |
| `new_without_default` | `installer/src/app.rs:382` | block 13 |

**Milestone checks, beyond the per-block gate:**

1. **Version gate**: `cargo run -p xtask -- version`. After block 7, confirm every crate still
   passes *and* that a deliberately malformed version now fails where it previously passed.
2. **Full signed build**: `cargo run -p xtask -- release`, then
   `cargo run -p xtask -- verify target/release/{launcher,listener,keeper,installer}.exe`. After
   block 7 this checks roles — confirm each binary reports its own role, and that a deliberately
   mis-signed binary now fails.
3. **Block 1 (manual)**: put a stray comma in a cartridge's `catalog.json` and run
   `launcher.exe`. Before: nothing happened at all. After: an empty shelf, and
   `logs/launcher.log` names the file and the parse error. Then mark the cartridge folder
   read-only and confirm it still opens.
4. **Blocks 3-5 end-to-end (manual)**: write a cartridge with the installer, unplug and replug
   it with the listener running, confirm from `listener.log` that it verified and launched.
   Click a cover; confirm `keeper.exe` starts,
   `%LOCALAPPDATA%\Romzeta\active_game_lease.txt` appears, and `games/<slug>/counter.txt` ticks.
5. **Blocks 8-9 + 15 (manual)**: both tray icons appear, show their menus, and remove themselves
   on exit; the keeper is one Task Manager entry under "Romzeta"; and all four exes show a real
   icon in Explorer.
6. **Blocks 10-13 (manual)**: walk the installer wizard both ways — create, then reopen to edit,
   add a game, remove a game, change the key — and cancel a copy midway to confirm the cancel
   path still reports "cancelled" rather than a generic failure.
7. **Graph**: `graphify update .` after block 14; confirm `runTest()` is no longer a god node.

---

## Progress log

One line per block, appended as each agent finishes and its verification passes. Nothing is
written here on the strength of an agent's own report — only after the main session has checked
the code and run the gate.

| # | Block | Status | Files changed | Gate result |
|---|---|---|---|---|
| 1 | Launcher fails soft | **done** | `launcher/src/{catalog,content,main}.rs`, new `launcher/tests/catalog_damage.rs` | tests 105/105 pass · clippy blocked by pre-existing `common/src/reg.rs` (see 1b) |
| 1b | `common::reg` raw-pointer lint | **done** | `common/src/reg.rs`, `launcher/src/steam.rs`, `installer/src/{autoplay,font,listener}.rs` | tests 105/105 · **clippy 0 errors — workspace compiles under clippy for the first time**; 6 pre-existing warnings left for blocks 9/12/13/14 |
| 2 | Create the contract | **done** | new `common/src/{cartridge,paths}.rs`, new `common/tests/cartridge.rs`, `common/src/lib.rs`, `common/Cargo.toml`, `common/tests/common/mod.rs` | tests **109**/109 (4 new, 27 checks) · clippy 0 errors · fmt clean · serde runtime edges 0 in listener/keeper/common, 3 with `--features catalog` |
| 3 | Installer adopts it | **done** | `installer/Cargo.toml`, `installer/src/{constants,catalog,listener,cartridge,app}.rs`, `installer/src/ui/mod.rs`, `installer/tests/catalog.rs` | tests 109/109 · clippy 0 errors · fmt clean but the known outlier. `constants.rs` keeps `LAUNCHER_NAME`/`KEEPER_NAME` as `pub use common::cartridge::…` rather than deleting them — `volume.rs` and `tests/volume.rs` were outside the block's remit |
| 4 | Launcher adopts it | **done** | `launcher/Cargo.toml`, `launcher/src/{catalog,config,content,keeper,log,assets,launch,ui}.rs`, `launcher/tests/catalog.rs` | tests 109/109 · clippy 0 errors · fmt clean but the known outlier. `Game` is now `pub type Game = common::cartridge::Entry`. Two intended behaviour changes shipped: strict `gameDir`/`isContained`, and `logs/<slug>/` on `_`. `launcher/src/constants.rs:LOG_FILE` stays a literal — a `const` cannot concatenate another crate's `const`; block 12's `Log` handle is where it goes |
| 5 | Listener + keeper adopt it | **done** | `common/src/lease.rs`, `listener/src/{constants,log,trust}.rs`, `keeper/src/{constants,main,run}.rs`, `launcher/src/ui.rs` | tests 109/109 · clippy 0 errors · fmt clean but the known outlier · serde runtime edges 0 in listener and keeper. `Lease.cartridge_root` dropped. `keeper/src/constants.rs`'s `LOG_FILE` became `fn logFile() -> PathBuf` built on `LOGS_DIR` |
| 6 | One catalog contract test | **done, narrowed** | new `common/tests/catalog.rs`, `common/tests/common/mod.rs`, `common/Cargo.toml` | tests **112**/112 (3 new, 30 checks) · clippy 0 errors · fmt clean but the known outlier. Could not link both crates — see the winres finding below |
| 7 | Trust + version single-sourced | **done** | new `trust/src/keyfile.rs`; `trust/src/{lib,constants}.rs`; `listener/{build.rs,Cargo.toml}`; `installer/build.rs`; `xtask/src/{keys,sign,main,manifest,release,constants}.rs`; `xtask/Cargo.toml`; `xtask/tests/{manifest,sign}.rs` | tests **113**/113 · clippy 0 errors · fmt clean but the known outlier. `xtask version` passes; all four release exes verify with their own role; a keeper renamed `launcher.exe` is now **REFUSED** where it passed before |
| 8 | Create `common::win32` | **done** | new `common/src/win32.rs`, `common/src/lib.rs`, `common/Cargo.toml` | tests 113/113 · clippy 0 errors, the 5 known warnings only · fmt clean but the known outlier. No call site changed, so the duplication is still there until block 9 — that is by design. An abrupt PC shutdown mid-block corrupted the dev build cache (`Unsupported archive identifier` on three rlibs); `cargo clean --profile dev` cleared it, no source lost |
| 9 | Three crates adopt win32 | **done** | `launcher/src/{tray,instance,constants}.rs`, `launcher/Cargo.toml`; `listener/src/trigger/windows.rs`, `listener/src/{constants}.rs`, `listener/Cargo.toml`; `keeper/src/window.rs`, `keeper/Cargo.toml` | tests 113/113 · clippy 0 errors, the 4 known sites only · **fmt completely clean for the first time**. −473 lines across the three crates. Menu ids became indices beside their labels; `Win32_System_LibraryLoader` dropped from all three crates and `Win32_Graphics_Gdi` from two |
| 10 | Split `installer/src/app.rs` | not started | — | — |
| 11 | Split `launcher/src/ui.rs` | not started | — | — |
| 12 | Launcher sweep | not started | — | — |
| 13 | Installer sweep | not started | — | — |
| 14 | Tooling sweep | not started | — | — |
| 14b | `catalog.json` gets one parser | not started | — | — |
| 15 | Icons | **blocked** — needs 3 `.ico` files | — | — |
| 16 | `README.md` | not started | — | — |
| 17 | `launcher/structure.md` | not started | — | — |
| 18 | `installer/structure.md` | not started | — | — |
| 19 | `listener/structure.md` | not started | — | — |
| 20 | `SIGNING.md` + `listener/README.md` | not started | — | — |
| 21 | The checklists | not started | — | — |

Version bumps are **not** part of any block. Per CLAUDE.md they happen when the user says
"done", and committing is never implied by finishing.
