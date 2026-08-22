// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Draws the games screen: the cartridge's name, the games already on it, and
//! the ones being added. A game in either list has the same four things to set,
//! so both draw [`details`].

// ########## THE GAMES SCREEN ##########

use std::path::Path;

use crate::app::{App, Details, Edit, KeeperState, Mode};
use crate::steam::Found;
use crate::version;
use crate::volume::humanBytes;

use crate::constants::{BAD, GOOD, STEAMDB_URL, WARN};

pub fn screen(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    name(app, ui);
    ui.separator();

    if app.mode == Mode::Edit {
        staleLauncher(app, ctx, ui);
        staleKeeper(app, ctx, ui);
        existing(app, ctx, ui);
        ui.separator();
    }

    ui.horizontal(|ui| {
        if ui.button("Add a game folder…").clicked()
            && let Some(folder) = rfd::FileDialog::new()
                .set_title("Pick the folder the game is installed in")
                .pick_folder()
        {
            app.addGame(ctx, folder);
        }
        ui.label(egui::RichText::new("The whole folder is copied onto the cartridge.").weak());
    });
    ui.add_space(8.0);

    let mut drop = None;
    for index in 0..app.drafts.len() {
        if draft(app, index, ui) {
            drop = Some(index);
        }
        ui.add_space(6.0);
    }
    if let Some(index) = drop {
        app.drafts.remove(index);
    }

    if app.drafts.is_empty() && app.mode == Mode::Create {
        ui.add_space(12.0);
        ui.label(egui::RichText::new("No games added yet.").weak());
    }
}

/// What the cartridge is called — the drive's volume label, and the only part of
/// a cartridge that isn't a file on it.
///
/// Nothing is written here: the new name rides along in the plan and is applied
/// with the rest, so backing out of this screen leaves the drive alone. A name
/// the filesystem won't take is reported by the footer, like every other thing
/// standing between here and Review.
fn name(app: &mut App, ui: &mut egui::Ui) {
    let limit = app.volume().map(|v| v.maxLabelLen()).unwrap_or(32);
    ui.horizontal(|ui| {
        ui.label("Cartridge name:");
        ui.add(egui::TextEdit::singleline(&mut app.name).desired_width(300.0));
    });
    ui.label(
        egui::RichText::new(format!(
            "The drive's name — what Windows shows beside it in Explorer. Up to {limit} \
             characters; leave it empty for no name."
        ))
        .weak()
        .small(),
    );
    ui.add_space(8.0);
}

/// Shown when the cartridge's `launcher.exe` states a version other than the
/// one this installer carries. An empty plan cannot reach Review
/// (`Plan::isEmpty`), so this is the only route to refreshing a launcher on a
/// cartridge whose games and name are otherwise fine.
fn staleLauncher(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let Some(theirs) = app.staleLauncher else {
        return;
    };
    let ours = version::bundled()
        .expect("staleLauncher is only set when both the probe and the bundled version parsed");
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.colored_label(
            WARN,
            format!(
                "This cartridge's launcher is version {theirs}, this installer carries {ours}."
            ),
        );
        if ui.button("Update launcher").clicked() {
            app.updateLauncher(ctx);
        }
    });
    ui.add_space(8.0);
}

/// The keeper counterpart to [`staleLauncher`] — shown when the cartridge
/// either has an out-of-date `keeper.exe` or, on a cartridge from before
/// keeper existed, none at all.
fn staleKeeper(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let Some(state) = app.staleKeeper else {
        return;
    };
    let ours = version::bundledKeeper()
        .expect("staleKeeper is only set when the bundled keeper version parsed");
    egui::Frame::group(ui.style()).show(ui, |ui| {
        let message = match state {
            KeeperState::Missing => format!(
                "This cartridge has no keeper — it predates one. This installer carries \
                 version {ours}."
            ),
            KeeperState::Stale(theirs) => format!(
                "This cartridge's keeper is version {theirs}, this installer carries {ours}."
            ),
        };
        ui.colored_label(WARN, message);
        if ui.button("Update keeper").clicked() {
            app.updateKeeper(ctx);
        }
    });
    ui.add_space(8.0);
}

/// What the cartridge already holds. Each game is one line until its Edit
/// button is pressed, so a cartridge with ten games stays a list rather than
/// ten stacked editors — and only the game opened pays for the folder walk its
/// executable picker needs.
fn existing(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    if app.existing.is_empty() {
        ui.label(egui::RichText::new("This cartridge has no games on it yet.").weak());
        ui.add_space(8.0);
        return;
    }
    ui.label(egui::RichText::new("Already on this cartridge").strong());
    ui.add_space(4.0);

    // Both kept the same length as `existing` here rather than at every call
    // site, so a catalog read that changed length can't panic the UI.
    app.remove.resize(app.existing.len(), false);
    app.edits.resize_with(app.existing.len(), || None);

    let root = app.volume().map(|v| v.root.clone());
    let mut error = None;
    // Deferred: `Edit::start` needs the context and a borrow of `existing` that
    // the row is already holding.
    let mut start = None;

    for index in 0..app.existing.len() {
        let App {
            existing,
            remove,
            edits,
            ..
        } = &mut *app;
        let entry = &existing[index];
        let removed = remove[index];
        let edit = &mut edits[index];

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut checked = removed;
                if ui.checkbox(&mut checked, "Remove").changed() {
                    remove[index] = checked;
                }
                ui.vertical(|ui| {
                    // The edited name if there is one: the row is what says
                    // whether a change took.
                    let shown = edit
                        .as_ref()
                        .map(|e| e.entry())
                        .unwrap_or_else(|| entry.clone());
                    let title = if removed {
                        egui::RichText::new(&shown.name).strikethrough().color(BAD)
                    } else {
                        egui::RichText::new(&shown.name)
                    };
                    ui.label(title);
                    ui.label(egui::RichText::new(&shown.exe).weak().small());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Nothing about a game being deleted is worth editing.
                    let enabled = !removed;
                    match edit {
                        Some(open) => {
                            let label = if open.open { "Done" } else { "Edit" };
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                open.open = !open.open;
                            }
                            if open.changed()
                                && ui
                                    .add_enabled(enabled, egui::Button::new("Revert"))
                                    .clicked()
                            {
                                *edit = None;
                            }
                        }
                        None => {
                            if ui.add_enabled(enabled, egui::Button::new("Edit")).clicked() {
                                start = Some(index);
                            }
                        }
                    }
                });
            });

            let Some(open) = edit else { return };
            if !open.open || removed {
                return;
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(open.dir.display().to_string())
                    .weak()
                    .small(),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut open.details.name).desired_width(300.0));
            });
            ui.add_space(6.0);

            let dir = open.dir.clone();
            let keeping = open.original.image.clone();
            if let Some(problem) =
                details(("edit", index), &dir, Some(&keeping), &mut open.details, ui)
            {
                error = Some(problem);
            }

            ui.add_space(6.0);
            match open.blocker() {
                Some(blocker) => ui.colored_label(WARN, blocker),
                None if open.changed() => ui.colored_label(GOOD, "Changed."),
                None => ui.label(egui::RichText::new("Unchanged.").weak()),
            };
        });
        ui.add_space(4.0);
    }

    if let Some(index) = start
        && let Some(root) = &root
    {
        app.edits[index] = Edit::start(ctx, root, &app.existing[index]);
        if app.edits[index].is_none() {
            error = Some(format!(
                "{} cannot be edited: its catalog paths do not name a folder on this cartridge.",
                app.existing[index].name
            ));
        }
    }
    if let Some(problem) = error {
        app.error = Some(problem);
    }

    if app.remove.iter().any(|r| *r) {
        ui.colored_label(
            WARN,
            "Removing deletes that game's folder and cover from the cartridge.",
        );
    }
    ui.add_space(8.0);
}

/// One game being added. Returns true when its Remove button was pressed.
fn draft(app: &mut App, index: usize, ui: &mut egui::Ui) -> bool {
    let mut dropped = false;
    let mut error = None;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        let draft = &mut app.drafts[index];

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(egui::TextEdit::singleline(&mut draft.details.name).desired_width(300.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Remove").clicked() {
                    dropped = true;
                }
            });
        });
        ui.label(
            egui::RichText::new(draft.source.display().to_string())
                .weak()
                .small(),
        );
        ui.add_space(6.0);

        let source = draft.source.clone();
        if let Some(problem) = details(("draft", index), &source, None, &mut draft.details, ui) {
            error = Some(problem);
        }
        if draft.details.scanning() {
            return;
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let scan = draft.details.scan.as_ref();
            let (bytes, files) = scan
                .map(|s| (s.total_bytes, s.file_count))
                .unwrap_or((0, 0));
            ui.label(
                egui::RichText::new(format!("{} in {files} files", humanBytes(bytes)))
                    .weak()
                    .small(),
            );
            match draft.blocker() {
                Some(blocker) => ui.colored_label(WARN, blocker),
                None => ui.colored_label(GOOD, "Ready."),
            };
        });
    });

    if let Some(e) = error {
        app.error = Some(e);
    }
    dropped
}

/// The executable, the Steam pair and the cover — everything a game carries
/// that is not its name, for a game in either list.
///
/// `keeping` is the cover the cartridge already holds, and is what makes an
/// empty picker mean "leave it alone" here and "not filled in yet" for a game
/// being added. Returns a problem to raise, if the user caused one.
fn details(
    salt: (&str, usize),
    folder: &Path,
    keeping: Option<&str>,
    d: &mut Details,
    ui: &mut egui::Ui,
) -> Option<String> {
    let mut error = None;

    if d.scanning() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Reading the folder…");
        });
        return None;
    }
    let Some(scan) = &d.scan else {
        ui.colored_label(BAD, "That folder could not be read.");
        return None;
    };
    let candidates = scan.candidates.clone();

    // ── Executable ──────────────────────────────────────────────────────
    // The exe's own folder is the first place an app id is looked for, so a new
    // exe is a reason to look again.
    let mut exe_changed = false;
    ui.horizontal(|ui| {
        ui.label("Starts:");
        let selected_text = d
            .exeRelative()
            .map(|p| crate::catalog::toRelativeString(&p))
            .unwrap_or_else(|| "— choose one —".into());

        egui::ComboBox::from_id_salt(("exe", salt))
            .selected_text(selected_text)
            .width(380.0)
            .show_ui(ui, |ui| {
                for (n, candidate) in candidates.iter().enumerate() {
                    let label =
                        format!("{} — {}", candidate.display(), humanBytes(candidate.bytes));
                    if ui.selectable_label(d.selected == Some(n), label).clicked() {
                        d.selected = Some(n);
                        d.manual_exe = None;
                        exe_changed = true;
                    }
                }
                if candidates.is_empty() {
                    ui.label(egui::RichText::new("Nothing was detected.").weak());
                }
            });

        if ui.button("Browse…").clicked()
            && let Some(chosen) = rfd::FileDialog::new()
                .set_title("Pick the executable to start")
                .set_directory(folder)
                .add_filter("Programs", &["exe"])
                .pick_file()
        {
            match d.setManualExe(folder, &chosen) {
                Ok(()) => exe_changed = true,
                Err(e) => error = Some(e),
            }
        }
    });
    if exe_changed && d.steam {
        d.detectAppid(folder);
    }

    if d.manual_exe.is_some() {
        ui.label(egui::RichText::new("Chosen by hand.").weak().small());
    } else if d.exeRelative().is_none() {
        // Nothing is selected and there is nothing to fall back to — which is
        // either an empty list or a list too close to call.
        ui.colored_label(
            WARN,
            match candidates.is_empty() {
                true => "No likely executable in that folder — use Browse to point at one.".into(),
                // Naming the count is what makes it clear this is a real choice
                // and not a failure.
                false => format!(
                    "{} executables look equally likely — pick the right one.",
                    candidates.len()
                ),
            },
        );
    }

    ui.add_space(6.0);

    // ── Steam ───────────────────────────────────────────────────────────
    // A property of what starts, so it sits with the exe rather than off in a
    // settings screen.
    if ui
        .checkbox(&mut d.steam, "Needs Steam — start it silently first")
        .changed()
        && d.steam
    {
        d.detectAppid(folder);
    }
    ui.label(
        egui::RichText::new(
            "For a game whose DRM refuses to run unless the Steam client is up. \
             The launcher starts it in the tray, with no window, and waits.",
        )
        .weak()
        .small(),
    );
    if d.steam {
        appid(salt, d, ui);
    }

    ui.add_space(6.0);

    // ── Cover ───────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Cover:");
        if ui.button("Choose an image…").clicked()
            && let Some(chosen) = rfd::FileDialog::new()
                .set_title("Pick this game's cover")
                .add_filter("Images", &["png", "webp", "jpg", "jpeg", "gif", "avif"])
                .pick_file()
        {
            d.setImage(chosen);
        }
        ui.label(egui::RichText::new("(?)").weak())
            .on_hover_text(format!(
                "{}×{} works best — the 2:3 shape the launcher lays out. Anything \
                 else is copied as it is, and shown at a different size to the rest.",
                crate::constants::TARGET_WIDTH,
                crate::constants::TARGET_HEIGHT
            ));
        match (&d.image, keeping) {
            (Some(path), _) => {
                ui.label(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
            }
            (None, Some(current)) => {
                ui.label(egui::RichText::new(format!("Keeping {current}")).weak());
            }
            (None, None) => {
                ui.colored_label(WARN, "None chosen.");
            }
        }
    });
    if let Some(warning) = &d.image_warning {
        // A warning, not a rejection: v1 copies the file as-is rather than
        // resizing it, and the launcher renders whatever shape it is given.
        ui.colored_label(WARN, warning);
    }

    error
}

/// The app id that goes in `steam_appid.txt` beside the exe, and where it came
/// from. Shown only while the Steam box is ticked.
fn appid(salt: (&str, usize), d: &mut Details, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Steam app id:");
        if ui
            .add(
                egui::TextEdit::singleline(&mut d.appid)
                    .id_salt(("appid", salt))
                    .desired_width(120.0),
            )
            .changed()
        {
            d.appid_found = None;
        }
        ui.hyperlink_to("steamdb.info", STEAMDB_URL);
    });
    match d.appid_found {
        Some(Found::File) => {
            ui.label(
                egui::RichText::new("From the steam_appid.txt the game already carries.")
                    .weak()
                    .small(),
            );
        }
        Some(Found::Manifest) => {
            ui.label(
                egui::RichText::new("From Steam's own record of this install.")
                    .weak()
                    .small(),
            );
        }
        // Typed, or nothing to type over yet. Only the empty case is a problem,
        // and the footer already names it — this says where to go about it.
        None if d.appid.trim().is_empty() => {
            ui.colored_label(
                WARN,
                "Not recorded on this PC — look the game up on steamdb.info and type its id in.",
            );
        }
        None => {}
    }
}
