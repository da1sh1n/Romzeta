# Romzeta Keeper — TODO

## Stats plan

- [ ] Split the live per-run counter from the persisted aggregate stats.
- [ ] Keep the active session in `counter.txt` and refresh it on each keepalive tick.
- [ ] Persist aggregate values in `stats.toml` with:
  - `playtime_seconds`
  - `last_played`
  - `played_count`
- [ ] On session end, add the live counter to `playtime_seconds`, update `last_played` to the current Unix timestamp, increment `played_count`, and reset `counter.txt`.
- [ ] Preserve the current `--playtime` override for custom file paths while defaulting to the cartridge root.
- [ ] Decide whether `stats.toml` is created lazily on first run or at install time.
- [ ] Add `first_played`, set once and never rewritten. The launcher's stats card shows it — see
      [`../launcher/TODO.md`](../launcher/TODO.md#stats-card--right-click).

## Achievements

With no Steam client running, `SteamAPI_Init` fails and an unlock happens nowhere. A `steam_api`
shim beside the game answers `SetAchievement` and `StoreStats` itself, so the cartridge is the only
record — it starts empty and Steam is never asked.

- [ ] The `steam_api` shim beside the game — answer the stats calls locally instead of failing.
- [ ] Record each unlock as an achievement id and a Unix timestamp.
- [ ] Decide the shape: a file of its own, or an `[achievements]` block in `stats.toml`.
