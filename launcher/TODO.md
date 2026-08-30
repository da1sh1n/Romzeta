# Romzeta Launcher — TODO

The launcher carries no secret and verifies nothing. Its signature *is* the cartridge's
identity, and it is read rather than asked — see
[`structure.md`](structure.md#role-in-cartridge-identification).

Launching, feedback, the gallery, ordering and the Departure Mono type grid are all built;
[`structure.md`](structure.md) describes what shipped.

## Type

The chrome is all one size. The next step up the 11px grid is 22px, which is name-plate
territory, so hierarchy in the toolbar has to come from the colour ramp and the pill fill
rather than from a size in between.

Nothing open.

## One game — skip the gallery

- [ ] A cartridge holding a single game launches it straight away, with no window at all.

A gallery of one cover is a menu with nothing to choose. [`src/main.rs`](src/main.rs) already has
the game list's worth of work done by the time it calls `ui::run` — the branch belongs there, after
`catalog::load`, before the window and the webview are built. The webview is the expensive half of
startup, so this is also the fastest the launcher can ever start.

`launch::spawn` plus `launch::supervise` on the main thread is enough here: nothing is repainting,
so there is no reason for the worker thread the gallery needs.

**A failed launch still opens the gallery.** `Outcome::Failed` is a line meant to sit under a
cover, and a launcher that exits silently on a broken cartridge tells the player nothing at all.
Fall through to `ui::run` and let the page show the error it already knows how to show.

Only the seeding and the single-instance lock run before the branch, and both must stay — a second
double-click should still do nothing rather than start the game twice.

## Achievement store — check at start

The keeper's store, read beside `catalog::load` in [`src/main.rs`](src/main.rs) before the webview
is built. Nothing here may delay startup.

- [ ] Read each game's `stats.toml` and hand it to the page alongside the catalog payload.
- [ ] `tracks_achievements` from the catalog says whether a game has a source at all — a game
      without one is not the same as one tracked with nothing unlocked yet.
- [ ] A missing or unreadable store means "no achievements yet", not an error under a cover.
- [ ] Flag a missing or stock `steam_api` beside a game whose flag is true — the next run would
      record nothing.

## Stats card — right click

Right-clicking a cover replaces it with its stats, in the same grid slot so the row never reflows.
Right-clicking again puts the cover back. [`src/cards.js`](src/cards.js), off the payload the
section above loads.

- [ ] `contextmenu` flips a cover to its stats card and back, in place.
- [ ] Time played, last played, first played — the keeper does not persist `first_played` yet.
- [ ] The achievement list under them, scrolling inside the card rather than growing it.
- [ ] No achievement half at all when `tracks_achievements` is false — never an empty list.
- [ ] Suppress the browser's own context menu, and keep Escape and the cursor ring working while a
      card is flipped.

## Saves system — parked until `2.x.y`

Games launched from a cartridge still write their saves to the PC. The goal is the opposite: a
`save/<slug>/<profile>/` tree on the cartridge itself, per profile, so one stick moves between
machines and between people without mixing anyone's progress.

Nothing on Windows redirects every game at once without a kernel driver, so this is five layers,
each catching a kind of game the one before it misses. Layer 0 is the game that already saves
beside its own exe and needs nothing done to it. Layer 1 junctions a known save folder on C: into
the cartridge, so the bytes never reach the system disk. Layer 2 carries registry saves in and out
around the run. Layer 3 catches games nothing knows about, by overriding the environment and then
adopting whatever folder appeared. Layer V is the opt-in one: a DLL injected into the game that
rewrites paths itself, reaching games no manifest describes.

Per game the order is `0 → 1 → V → (2, 3)` with deep mode on, `0 → 1 → 2 → 3` without it. Layers
0–3 never elevate. Known save locations come from the Ludusavi manifest (MIT, compiled from
PCGamingWiki), embedded and resolved by the installer — see [`../installer/TODO.md`](../installer/TODO.md).

**Why `2.x.y`:** points 7 and 8 add a launcher↔listener IPC protocol, and the listener refuses to
run a launcher whose `x` differs from its own. That is precisely what `project_version` in the
workspace `Cargo.toml` gates. The workspace is at `0`; this waits.

- [ ] 1. One shared `slug()` in `common`, so `save/<slug>` and `games/<slug>` cannot drift apart.
- [ ] 2. Profiles — `profile.rs`, the `active_profile` key, a selector in the gallery.
- [ ] 3. The `save` block in `catalog.json`: parse it, and validate it as untrusted input.
- [ ] 4. Layer 1 — junctions: resolve placeholders, create and verify, adopt what is already there.
- [ ] 5. Layer 2 — registry: import before the run, export and wipe after it.
- [ ] 6. Layer 3 — environment override, then diff and adopt whatever the game created.
- [ ] 7. Suspended spawn, Job Object, hand it to the listener, fall back to `--wait`.
- [ ] 8. `--game_closed` — the headless mode that does the post-run work.
- [ ] 9. The save-layer badge on every cover.
- [ ] 10. Layer V — the `hook` crate and its injected DLL. Largest; leave it last.

### 1. One shared `slug()` in `common`

Two of them exist and they disagree: [`../installer/src/catalog.rs`](../installer/src/catalog.rs)
separates with `_`, [`src/log.rs`](src/log.rs) with `-`. The same game name comes out as two
different strings. `save/<slug>/` has to line up with `games/<slug>/`, which the installer writes,
so promote the installer's into `common` and have both crates call it.

Leave the log slug alone. `logs/<slug>/` is already on disk with hyphens and renaming it buys
nothing. Do this first — points 3 through 6 all key their paths off the result.

### 2. Profiles

New `profile.rs`: read and write `active_profile`, list and create profiles, seed the default from
the Windows username on first run so nobody has to configure anything to get started. The key goes
into `config.toml` through the existing `syncDefaults` migration in [`src/config.rs`](src/config.rs),
which is how a cartridge written by an older installer picks up a key it has never seen.

A profile is a directory under `save/`, so a profile name is user input that becomes a path — it
goes through the same slugging and validation as anything else that does. The selector rides the
IPC already in [`src/ui.rs`](src/ui.rs).

Config writes already tolerate a write-protected stick; keep that true here.

### 3. The `save` block in `catalog.json`

Each entry gains an optional `"save": { "files": [...], "registry": [...] }`. Field names and types
match what the installer writes, the same hard contract the existing fields already have.

`catalog.json` is untrusted input — [`structure.md`](structure.md) says so and
[`src/catalog.rs`](src/catalog.rs) already refuses `..` for exactly this reason. Validation here:
only the known placeholders (`<base>`, `<home>`, `<winAppData>`, `<winLocalAppData>`,
`<winDocuments>`), no `..`, at least two path components below the known folder, and a denylist for
the `Microsoft` and `Windows` subtrees. Without that last rule a hand-edited catalog could junction
a system folder onto the cartridge. Registry entries: `HKCU` only, reject anything else.

### 4. Layer 1 — junctions

Resolve placeholders at launch, never at install time: `SHGetKnownFolderPath`, so a Documents
folder redirected into OneDrive and a different drive letter on the next PC both come out right.

A junction is `FSCTL_SET_REPARSE_POINT` and needs no elevation. It sits on the C:/NTFS side, so the
cartridge itself can stay exFAT or FAT32. Rebuild it before every launch — it self-heals, and it
absorbs the drive letter changing between machines.

Adoption on first run: if the real folder already holds saves, move them onto the cartridge before
junctioning. Skip this and the player loses their existing progress the first time they use the
feature. If the path is already our junction, leave it alone.

Every failure in this layer is non-fatal — log it and launch the game anyway. A cartridge can be
write-protected, which the config code already treats as ordinary.

### 5. Layer 2 — registry

`reg.exe` import and export, `HKCU` only, no elevation. Import `registry.reg` before the run if it
exists; after the run export the keys the catalog names, then delete them from `HKCU` so nothing is
left behind on the machine.

This is the one layer whose data sits on C: while the game runs. That is unavoidable without
injection — say so plainly rather than implying the layer is airtight.

If the export fails, do not delete. Leaving keys on the machine is recoverable; losing them is not.

The export half runs from point 8, so write it as something that part can call.

### 6. Layer 3 — unknown games

Two nets. First, set `APPDATA` / `LOCALAPPDATA` / `USERPROFILE` / `HOME` on the child process to
`save/<slug>/<profile>/home/`. [`src/launch.rs`](src/launch.rs) sets no environment at all today, so
this is new ground. It only works for games that read the environment rather than asking the shell,
which is a minority — hence the second net.

Second, snapshot the common save roots before the run and diff after it. Any folder that appeared
gets recorded into the catalog's `save` block, and the next launch junctions it under layer 1. The
first session leaks to C:; every session after it lands on the cartridge.

Keep the snapshot cheap — top-level directory names and mtimes, not a full tree walk. A real
`AppData` is enormous.

### 7. Suspended spawn, Job Object, handover

Today [`src/launch.rs`](src/launch.rs) spawns the game and then drops the `Child`; nothing ever
observes it exiting, and the launcher closes itself. Layers 2, 3 and V all need someone watching.

Sequence: create the game **suspended**, create a Job Object, assign the game to it, hand it over,
then resume. Suspended is what makes it race-free — a game that immediately spawns its real exe
would otherwise escape before anything was watching.

The handover is `WM_COPYDATA` to the listener's hidden window. No new dependency; `FindWindow`
doubles as the presence check and the return value is the ack. Carry a protocol version so an old
listener refuses cleanly instead of hanging.

Fall back to re-invoking ourselves as `launcher.exe --wait` when nothing answers — no listener
installed, listener stopped, protocol too old. That mode creates no webview. The running game
already pins the cartridge, so the extra process costs nothing in ejectability.

The other half of this is points 1–3 of [`../listener/TODO.md`](../listener/TODO.md).

### 8. `--game_closed`

Headless, `CREATE_NO_WINDOW`, no webview built. Does the registry export and wipe from point 5 and
the adoption from point 6, logging into `logs/<game-slug>/`.

The listener invokes it with a spec on the command line, so the arguments are untrusted the same way
`catalog.json` is: derive the cartridge root from the running binary's own location and check the
argument against it rather than believing what was passed.

`--wait` from point 7 is this same work with the waiting attached. One code path, two entry points.

### 9. The save-layer badge

A filled circle in the bottom-right corner of every cover. The colours are baked in and never
configurable — they are an identity, not a theme:

| Layer | Colour | Meaning |
| --- | --- | --- |
| 0 | `#009E73` | already portable, saves beside the exe |
| 1 | `#F0E442` | junctioned, never touches C: |
| 2 | `#E69F00` | registry, transits C: while playing |
| 3 | `#CC79A7` | unknown game, first session leaks to C: |
| V | `#000000` | virtualized by the injected DLL |

Worst layer wins: a game with both file and registry saves shows `2`. Severity runs `0 < 1 < 2 < 3`.
With deep mode on, anything that would be 2 or 3 shows V instead, matching the launch order.

V is a **prediction** — injection can still be refused at runtime and the run falls back to 2/3. The
badge says what was intended, not what happened.

The value rides in `payload` in [`src/catalog.rs`](src/catalog.rs), beside the `available` bool it
already computes per game. Markup goes in the `card-template` in [`src/index.html`](src/index.html),
gets set in [`src/cards.js`](src/cards.js) next to the existing `available` and `.note` handling,
and is styled in [`src/style.css`](src/style.css) — `.save-badge`, with the five colours as
`--save-layer-0` through `--save-layer-v`. `data-save-layer` ↔ `dataset.saveLayer` is the DOM's own
casing, the one contract we do not get to name.

Give every circle a white ring and a soft shadow. Without it the black badge vanishes on dark art
and the yellow one on bright art. A `title` names the layer in words; a bare coloured dot explains
nothing to anyone.

### 10. Layer V — the `hook` crate

A new cdylib crate shipped as `hook.dll` at the cartridge root, signed like everything else and
verified through `trust` before it is ever injected.

It inline-hooks the file and registry open calls and rewrites paths under the known save roots into
`save/<slug>/<profile>/`. The target root arrives through an environment variable the launcher sets
on the child. Injection happens while the game is suspended, in point 7's sequence, before resume.

Active only when the launcher is elevated **and** the installer's deep-mode checkbox was ticked.
Injecting into our own child does not strictly require admin; elevation is there for games that
raise themselves, and as the explicit gate on the invasive tier.

Expect anti-cheat and DRM to refuse it. That is why it is opt-in, why layers 0–3 stay the baseline
underneath it, and why this is the last point rather than the first.

Needs `retour` for the inline hooking — hook crate only, never linked into the launcher itself, and
subject to approval before it is added. Start narrow: redirect the game's own save roots, not a
general filesystem virtualization.
