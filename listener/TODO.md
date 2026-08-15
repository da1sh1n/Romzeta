# Romzeta Listener — TODO

Actionable list for the service specced in [`structure.md`](structure.md).
**The shared core and the Windows trigger are built and verified on real hardware; Linux is
not started.**

The two platforms differ in **process lifetime**, not just in which API detects a volume —
Windows stays resident from login to logout, Linux runs one-shot from udev and exits. So the
build splits into one shared core plus two unrelated triggers. See
[Execution models](structure.md#execution-models) before starting the Linux trigger.

## Linux trigger — not started

Placeholder at [`src/trigger/linux.rs`](src/trigger/linux.rs); the crate compiles on Linux and
`--check <mountpoint>` already exercises the shared core there.

- [ ] udev rule: `ACTION=="add"`, `SUBSYSTEM=="block"`, `ENV{ID_FS_USAGE}=="filesystem"`.
- [ ] `RUN+="… systemd-run --no-block …"` handoff — udev kills `RUN+=` children
      unconditionally when the event finishes, so it cannot run the work itself.
- [ ] Bounded wait for the mountpoint: udev fires on **device add**, before udisks2 mounts
      the filesystem. Give up cleanly on timeout.
- [ ] Resolve the active graphical session via logind (`loginctl`) and start the core with
      `systemd-run --uid=<user> --setenv=…` (`DISPLAY` / `WAYLAND_DISPLAY` /
      `DBUS_SESSION_BUS_ADDRESS`) — udev runs as root with no session.
- [ ] Decide and implement the headless case (no active session → log and exit).
- [ ] Verify nothing stays resident once the launcher is running.
- [ ] Decide the Linux equivalent of the drive-type filter — the Windows build drops network
      and virtual volumes before touching them, and `/media/...` vs an SMB mount needs the
      same call.

## Saves system — parked until `2.x.y`

The cartridge is growing a `save/<slug>/<profile>/` tree of its own, and some of the layers that
fill it can only finish once the game has exited — registry keys to export, a folder to notice and
adopt. Something has to be alive to see that happen, and this is already the process that is: it
sits on a message pump from login to logout.

So the listener **waits** and the launcher **acts**. When the game's process tree is gone, the
listener re-invokes the cartridge's own `launcher.exe --game_closed`. Nothing about the save layout
lives here — a listener installed months ago never needs to understand a cartridge written last
week, and no launcher process has to linger in RAM through an entire gaming session just to hold a
handle.

The launcher half is points 7 and 8 of [`../launcher/TODO.md`](../launcher/TODO.md); the design in
full is there.

**Why `2.x.y`:** this is a new launcher↔listener protocol, and the listener already refuses to run
a launcher whose `x` differs from its own. That is what `project_version` in the workspace
`Cargo.toml` is for.

- [ ] 1. `WM_COPYDATA` receiver — protocol version, and the UIPI filter that makes it arrive.
- [ ] 2. Duplicate the job handle and wait for the whole process tree to end.
- [ ] 3. On exit, re-verify and spawn `launcher.exe --game_closed`.
- [ ] 4. Elevated autostart for deep mode, and the settings flag that records it.

### 1. `WM_COPYDATA` receiver

A handler on the hidden window already pumping messages in
[`src/trigger/windows.rs`](src/trigger/windows.rs). The payload carries a protocol version, the job
handle value, the launcher's PID, the cartridge root, the slug, the profile, and the mode.

Check the version first and refuse cleanly if it is one we do not speak. The launcher falls back to
waiting on its own, so a refusal has to be immediate and unambiguous — never a silence it has to
time out on. The message's return value is the ack.

Call `ChangeWindowMessageFilterEx` to let `WM_COPYDATA` through UIPI. With deep mode on this process
runs elevated, and Windows silently drops window messages from a lower-integrity sender — which is
exactly what a launcher the user started by double-clicking is. *Silently* is the whole problem:
without this the feature simply stops working and nothing anywhere reports why.

### 2. Duplicate the job handle, wait for the tree

`DuplicateHandle` the job out of the launcher process, then wait for
`JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO` on an IO completion port.

That message is the reason this is a Job Object and not a process handle: it fires when the whole
tree is gone, so a launcher-style game whose first exe quits a second after starting is still
covered. Waiting on the process we were handed would call that game finished before it began.

The launcher exits right after the handover, so the duplicated handle is what keeps the job alive.

More than one game can be running, and more than one cartridge can be connected. Keep a table keyed
by job — one global here would break the second cartridge.

### 3. Re-verify, then spawn `--game_closed`

On completion, start `launcher.exe --game_closed` with the spec from the message.

Go through `verifyLauncher` in [`src/trust.rs`](src/trust.rs), exactly as an ordinary cartridge
start does. A window message can come from any process on the machine, and this one names a binary
to execute. Attestation is the entire reason that is safe. Skipping it because this launcher was
verified a few hours ago is how it stops being safe — verify at the moment of execution, from the
bytes on disk, every time.

If the cartridge is gone by then, log it and drop the work. The saves have nowhere to land and there
is nothing to recover.

### 4. Elevated autostart, and the settings flag

Deep mode changes how this process starts: a Scheduled Task at logon with highest privileges instead
of the `HKCU\…\Run` key, which is the standard way to autostart elevated without a UAC prompt at
every login. The launcher inherits that token, and that is what puts layer V within reach.

Record which mode we are in via [`src/settings.rs`](src/settings.rs) — the process needs to know
whether to advertise deep mode when the launcher asks.

The installer is what writes it; see point 4 of [`../installer/TODO.md`](../installer/TODO.md).
Uninstall has to remove the task as well as the Run key, or a listener lingers with no way to
remove it from the UI.
