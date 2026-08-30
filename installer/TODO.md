# Romzeta Installer — TODO

The installer specced in [`structure.md`](structure.md) is **built** and has been exercised end
to end on real media. What is left is the Linux target.

## Future

- [ ] Theme screen: edit the launcher's `config.toml` from the wizard, with a live preview.
- [ ] Launch options per game, in Steam's syntax — extra arguments and `%command%`.
- [ ] Transfer speed and estimated time left on the working screen.
- [ ] Resume a failed or cancelled write instead of copying every game again.
- [ ] A button that checks GitHub releases for a newer installer, and the releases to check against.
- [ ] Detect whether a game can track achievements when it is added, and record it in the catalog.
- [ ] Linux target: `/opt` or `~/.local/share` instead of Program Files, a **udev rule**
      instead of a Run key (not a systemd user service — nothing runs between connections),
      Linux launcher binary instead of `launcher.exe`. `volume.rs` and `listener.rs` are the
      two modules with `#[cfg(windows)]` platform halves; everything else is portable
      already.

### Theme screen

A wizard screen with one control per key in [`../launcher/src/config.toml`](../launcher/src/config.toml)
— color pickers for the `*_color` keys, sliders for the gaps, sizes, radii and the `0..1` values,
checkboxes for `show_captions` and `show_console_window`, dropdowns for `background_effect` and
`order_mode`. `usage_order` and `user_order` are not the user's to set here; they stay out.

Beside the controls, a preview drawn in **egui** — no webview, and no second copy of the launcher's
renderer. A still mock of a launcher row is enough: three cover plates with their shadows, opacity
and corner radius, one of them selected and captioned, the cursor ring, the toolbar text, a loading
line, an error border, a dimmed missing-game plate. It reads the same values the launcher does, so
the point is that a number moves and the picture moves with it — not pixel fidelity.

**`background_effect` is not previewed.** Particles and fog are animated, and reproducing them here
means writing them twice. The dropdown sets the key; the preview paints `primary_color` flat behind
the covers and says the effect is not shown.

Writing it: [`src/cartridge.rs`](src/cartridge.rs) currently copies `payload::LAUNCHER_CONFIG`
verbatim and only when the file is absent, deliberately — the config belongs to whoever owns the
cartridge. This screen has to serialize the edited values instead, keep the comments in the shipped
file, and still refuse to clobber an existing `config.toml` without the user saying so. Editing an
already-written cartridge means reading its config back in first.

### Launch options per game

A text field beside each game on the games screen ([`src/ui/games.rs`](src/ui/games.rs)), holding
what Steam calls launch options, and behaving the way Steam's does so nobody has to learn a second
syntax:

- Plain text is split into arguments — whitespace-separated, quotes respected — and appended to the
  game's own command line. No shell: nothing expands `*`, `&&` or `%VAR%`.
- `%command%` anywhere in the string is replaced by the game's exe. What sits before it becomes the
  program actually run, with the exe as its argument; what sits after is passed to the game. That is
  the whole point of the token — it is how a wrapper gets in front of a game.
- Steam's Linux-only `VAR=value %command%` prefix form is not supported. Say so in the field's hint
  rather than accepting it and dropping it.

Stored as an optional `args` string on `Entry` in [`src/catalog.rs`](src/catalog.rs), written only
when it is non-empty so an untouched cartridge's `catalog.json` is unchanged. Field name and type
match the launcher's `Game`, the same hard contract the existing fields are under.

The launcher half is the real work: [`../launcher/src/launch.rs`](../launcher/src/launch.rs) builds
`Command::new(&exe)` with no arguments at all today. It has to parse the string, and — because
`catalog.json` is untrusted input there — run the wrapper named before `%command%` through the same
`isContained` check the exe already gets. Without that, one edited catalog line runs any program on
the machine. Not listed in [`../launcher/TODO.md`](../launcher/TODO.md) yet.

### Transfer speed and time left

`Progress` in [`src/cartridge.rs`](src/cartridge.rs) already carries `done` and `total`; the working
screen shows only the fraction. Add bytes per second and an estimate of the time remaining.

Compute it from a rolling window, not from the whole run — copying a hundred small files and copying
one large one differ by an order of magnitude, and a since-the-start average hides the change. The
estimate is `(total - done) / rate` and it will be wrong at the start; showing nothing until the
first window fills is better than showing a number that swings.

The `report` closure is called once per chunk, which is far more often than a display should
change. Sample on a timer instead — recompute the shown figures a few times a second and leave them
alone in between, or the text is unreadable.

### Resuming a failed write

A write that dies on the last of six games throws away the five that copied fine. `unwind()` in
[`src/cartridge.rs`](src/cartridge.rs) deletes every path in `created`, and `created` holds the
whole run. Pressing Write again starts from nothing.

Only the game in flight is unsafe — the ones before it are whole. Narrow `unwind()` to that game's
folder and cover, and leave the finished ones on the drive. Half a game folder is still worse than
none; that reasoning holds for one game, not for the run.

**Write `catalog.json` after each game rather than once at the end.** That is what makes resuming
need no new state. Each write names only games already fully copied, so the ordering guarantee
`apply()` is built on survives intact, and a failed run leaves a smaller but *correct* cartridge.
The next run reads it back through the path that already exists: the copied games appear in
`app.existing`, drop out of `add`, and the plan is the remainder. Nothing is persisted about the
interrupted run, and a user who never comes back is left with a working cartridge instead of an
invisible half-written one. The cost is N small catalog writes instead of one.

Cancel gets the same behaviour for free, which is a change in what cancel means — today it promises
the cartridge is exactly as it was. Say so on the working screen, and reword the guarantee in
`apply()`'s comment along with it.

Resuming *within* one game is a second step and only worth it for the case that hurts: a 60 GB game
failing at 55. It belongs in `copy::directory` — skip a destination file whose size matches the
source, delete the one file that was in flight rather than the folder. Size matching is not proof of
identical content, so it is a deliberate trade, and the interrupted file is the one case it would
get wrong, which is exactly the file being deleted.

The UI half: a failed write lands on Done with the error in `outcome`
([`src/app.rs`](src/app.rs)). That screen needs a Retry that rebuilds the plan against the drive as
it now stands, instead of sending the user back through the wizard.

## Updates from GitHub releases

`updateLauncher` and `updateKeeper` in [`src/app.rs`](src/app.rs) refresh a cartridge from the
binaries embedded here. What is missing is upstream: knowing a newer installer exists. This is the
only place the installer touches the network, and only on the button.

- [ ] Publish signed releases. `cargo run -p xtask -- release` already builds and signs; what is
      left is tagging and uploading the artifacts.
- [ ] A "Check for updates" button beside the existing two in [`src/ui/games.rs`](src/ui/games.rs).
- [ ] Compare the latest tag against [`src/version.rs`](src/version.rs).
- [ ] Verify a download against `keys/romzeta.pub` before offering to run it — the check
      `xtask verify` does. An unsigned one is not offered.
- [ ] Decide whether it replaces itself or just opens the release page.

## Achievements

Steam is the only source, and the answer is a file beside the exe — which `detect::scan` already
walks past. Deciding here is what spares the launcher an undecided state to resolve later.

- [ ] A marker list in [`src/constants.rs`](src/constants.rs): `steam_api64.dll` and `steam_api.dll`
      to begin with. Filenames, not libraries — a new marker is a line in the list.
- [ ] Look for one beside the exe during `detect::scan` in [`src/detect.rs`](src/detect.rs).
- [ ] `tracks_achievements` on `Entry` in [`src/catalog.rs`](src/catalog.rs), written from that
      check. `#[serde(default, skip_serializing_if = "isFalse")]`, the shape `steam` already uses.
- [ ] "Achievements tracked on this game." under the app id in
      [`src/ui/games.rs`](src/ui/games.rs). A label, not a checkbox.

## Saves system — parked until `2.x.y`

The cartridge is growing a `save/<slug>/<profile>/` tree, and the installer is the piece that knows
*where* each game keeps its saves. It embeds the Ludusavi manifest (MIT, compiled from
PCGamingWiki — file paths and registry keys for over ten thousand games), matches the games being
written to a cartridge, and records the answer in `catalog.json`. After that the launcher never
needs the network, or the full database, or any of this machinery.

It also owns the one user-facing decision in the whole feature: whether to allow the invasive tier.

The five layers and what they each catch are described in
[`../launcher/TODO.md`](../launcher/TODO.md).

**Why `2.x.y`:** the launcher and listener grow a shared IPC protocol, and the listener refuses to
run a launcher whose `x` differs. See `project_version` in the workspace `Cargo.toml`.

- [ ] 1. `xtask manifest` — fetch, trim and compress the Ludusavi manifest.
- [ ] 2. Embed it, match each game, write the `save` blocks into `catalog.json`.
- [ ] 3. `save/` in the cartridge layout, and the signed `hook.dll` beside the launcher.
- [ ] 4. The deep-mode checkbox, and the autostart branch behind it.

### 1. `xtask manifest`

`cargo run -p xtask -- manifest`: fetch the upstream manifest, keep `files`, `registry` and
`installDir`, and write it back out as gzipped JSON for the installer to embed. It is 16.7 MB raw
and 2.27 MB gzipped; trimmed it should land around 1–2 MB.

Keep the **Linux** entries as well as the Windows ones, with their `when` conditions intact. Linux
is on the roadmap for both other programs, the `<xdgData>` and `<xdgConfig>` paths cost very little
now, and dropping them means an installer data-format migration later instead.

Cache on the ETag: store what came back, send `If-None-Match`, rewrite only on a `200`.

Parsing is YAML, which is exactly why this lives in xtask — `serde_yaml` never ends up inside a
shipped binary. It needs `serde_yaml` here and `flate2` for the compression, both subject to
approval before they are added. If xtask has no HTTP client to reuse, taking an already-downloaded
path as an argument is a fine first version and costs no dependency at all.

### 2. Embed, match, write

`include_bytes!` the gzipped manifest and gunzip it at run time.

Match each detected game by install-directory name first — that is what the manifest's `installDir`
exists for — then fall back to a fuzzy title match. Write the resolved `save` block into
`catalog.json` alongside the fields already written in [`src/catalog.rs`](src/catalog.rs). Field
names and types match the launcher's `Game` exactly, the same hard contract the existing fields are
under.

A game that matches nothing gets no `save` block, and layer 3 picks it up at run time. That is the
designed outcome, not a failure to report.

Document the hand-edit path in [`structure.md`](structure.md): someone with a game too obscure for
the manifest should be able to write the block themselves.

### 3. `save/` and `hook.dll`

The layout writer in [`src/cartridge.rs`](src/cartridge.rs) gains `save/`. Copy the signed
`hook.dll` to the cartridge root beside `launcher.exe`.

Write `hook.dll` **always**, not only when the checkbox is ticked. Deep mode is a property of the PC
the cartridge is plugged into, not of the cartridge — the same stick may meet a machine that allows
it and one that does not.

Both [`structure.md`](structure.md) and the launcher's describe the cartridge layout and need the
same addition.

### 4. The deep-mode checkbox

A checkbox in the wizard — deep save redirection, requires elevation, **unchecked by default**.
Ticked, the listener is installed as a Scheduled Task running at logon with highest privileges
instead of the `HKCU\…\Run` key written by [`src/listener.rs`](src/listener.rs), and the choice is
recorded in listener settings. Uninstall removes the task.

This reverses a stance the project currently states outright: [`structure.md`](structure.md) has a
whole section saying nothing here elevates, and [`Cargo.toml`](Cargo.toml) carries the same line.
Both have to be rewritten to say nothing elevates *unless the user asks for it*, or the code and the
documentation will contradict each other.

The wizard has to say plainly what the checkbox buys and what it costs: saves captured for games no
other layer can reach, at the price of injecting a DLL that anti-cheat may well refuse. Someone
ticking it deserves to know both halves.
