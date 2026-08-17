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
