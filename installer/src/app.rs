// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Holds the wizard's state: the current screen, the chosen volume, the games
//! being added or removed, and the running job. `chooseVolume` is where the
//! create-vs-edit routing decision is made.

// ########## WIZARD STATE ##########

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use common::version::Version;

use crate::autoplay;
use crate::cartridge::{self, EditedGame, Plan, PlannedGame};
use crate::catalog::{self, Entry};
use crate::copy;
use crate::detect;
use crate::image;
use crate::listener;
use crate::payload;
use crate::steam;
use crate::version;
use crate::volume::{self, Volume};
use crate::wake;
use crate::work::{Job, Scanning};

/// Why the games screen would offer to refresh `keeper.exe` — see
/// `App::staleKeeper`.
#[derive(Clone, Copy)]
pub enum KeeperState {
    /// A cartridge from before keeper existed: a launcher, but no keeper.exe.
    Missing,
    /// A keeper.exe is there, but states a version other than
    /// `version::bundledKeeper()`.
    Stale(Version),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Screen {
    Home,
    /// Pick the volume; routes to create or edit.
    Volume,
    Games,
    Review,
    /// A job is running. The only screen with no way back.
    Working,
    Done,
    /// Job 2, reachable from Home and independent of the cartridge flow.
    Listener,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Create,
    Edit,
}

/// Everything about a game a person gets to set: its name, which file starts
/// it, whether Steam has to be up first, and its cover.
///
/// The same four either way, so a game being added and a game already on the
/// cartridge share one struct and one set of controls — see
/// [`crate::ui::games::details`]. What differs is only the folder they point at
/// and whether there is already an answer to fall back to.
pub struct Details {
    pub name: String,
    /// Running until the folder walk finishes.
    pub scanning: Option<Scanning>,
    pub scan: Option<detect::Scan>,
    /// Index into `scan.candidates`, or `None` when the exe came from Browse.
    pub selected: Option<usize>,
    /// Set only by the manual override; relative to the game folder.
    pub manual_exe: Option<PathBuf>,
    /// The exe the catalog already names, for a game on the cartridge. `None`
    /// while one is being added — there is nothing to fall back to yet.
    pub exe_fallback: Option<PathBuf>,
    /// A cover from the user's disk. For a game already on the cartridge,
    /// `None` means the one it has stays.
    pub image: Option<PathBuf>,
    /// The 2:3 note, if the chosen cover isn't that shape.
    pub image_warning: Option<String>,
    /// Whether this game's DRM needs the Steam client up before it will run.
    pub steam: bool,
    /// The app id that goes in `steam_appid.txt`. Text rather than a number, so
    /// a half-typed one is not reinterpreted between keystrokes.
    pub appid: String,
    /// Where `appid` was read from, or `None` once it was typed by hand.
    pub appid_found: Option<steam::Found>,
}

impl Details {
    /// Starts the folder walk and fills in what can be known without it.
    fn new(ctx: &egui::Context, folder: &Path, exe_fallback: Option<PathBuf>) -> Details {
        Details {
            name: detect::defaultName(folder),
            scanning: Some(Scanning::start(ctx, folder.to_path_buf())),
            scan: None,
            selected: None,
            manual_exe: None,
            exe_fallback,
            image: None,
            image_warning: None,
            steam: false,
            appid: String::new(),
            appid_found: None,
        }
    }

    /// Picks up the finished scan and decides what the exe field should say.
    pub fn poll(&mut self) {
        let Some(scanning) = &self.scanning else {
            return;
        };
        let Some(scan) = scanning.take() else { return };
        self.scanning = None;
        self.selected = match &self.exe_fallback {
            // A game that already starts something: find that file in the list
            // rather than second-guessing a decision already made.
            Some(current) => scan.candidates.iter().position(|c| c.relative == *current),
            // Only a *clear* winner is preselected. When the top two are too
            // close to call, the field is left empty and the user has to
            // choose — a guess presented as a decision is worse than no guess.
            None => scan
                .clearWinner()
                .and_then(|winner| scan.candidates.iter().position(|c| c == winner)),
        };
        self.scan = Some(scan);
    }

    pub fn scanning(&self) -> bool {
        self.scanning.is_some()
    }

    /// The executable, relative to the game folder.
    pub fn exeRelative(&self) -> Option<PathBuf> {
        if let Some(manual) = &self.manual_exe {
            return Some(manual.clone());
        }
        if let Some(scan) = &self.scan
            && let Some(candidate) = self.selected.and_then(|n| scan.candidates.get(n))
        {
            return Some(candidate.relative.clone());
        }
        self.exe_fallback.clone()
    }

    /// Accepts a hand-picked exe, which must be inside the game folder — the
    /// copy only moves that folder, so an exe from anywhere else would be a
    /// catalog entry pointing at a file that never shipped.
    pub fn setManualExe(&mut self, folder: &Path, chosen: &Path) -> Result<(), String> {
        let relative = chosen
            .strip_prefix(folder)
            .map_err(|_| format!("Pick an executable inside {}.", folder.display()))?;
        self.manual_exe = Some(relative.to_path_buf());
        self.selected = None;
        Ok(())
    }

    pub fn setImage(&mut self, chosen: PathBuf) {
        self.image_warning = image::ratioWarning(&chosen);
        self.image = Some(chosen);
    }

    /// The app id as a number, or `None` while the field is empty or not one.
    pub fn appidValue(&self) -> Option<u32> {
        steam::parse(&self.appid)
    }

    /// Fills the app id from disk, leaving anything already in the field alone.
    ///
    /// Called on a click rather than every frame — it is only a couple of file
    /// reads, but they are file reads.
    pub fn detectAppid(&mut self, folder: &Path) {
        if !self.appid.trim().is_empty() {
            return;
        }
        if let Some((appid, found)) = steam::detect(folder, self.exeRelative().as_deref()) {
            self.appid = appid.to_string();
            self.appid_found = Some(found);
        }
    }

    /// What still has to be filled in, in one sentence, or `None` when ready.
    ///
    /// `cover_required` is false for a game already on the cartridge: it has a
    /// cover, and leaving the picker alone keeps it.
    pub fn blocker(&self, cover_required: bool) -> Option<String> {
        if self.scanning() {
            return Some("still being read".into());
        }
        if self.name.trim().is_empty() {
            return Some("needs a name".into());
        }
        if self.exeRelative().is_none() {
            return Some(match self.scan.as_ref().map(|s| s.candidates.is_empty()) {
                Some(true) => "no executable was found — pick one".into(),
                _ => "needs an executable".into(),
            });
        }
        // With the box ticked and no id, the copy would put no steam_appid.txt
        // on the cartridge and the game would fail its handshake there — so the
        // id is a blocker exactly like the exe, not a warning.
        if self.steam && self.appidValue().is_none() {
            return Some(match self.appid.trim().is_empty() {
                true => "needs a Steam app id".into(),
                false => "has a Steam app id that isn't a number".into(),
            });
        }
        if cover_required && self.image.is_none() {
            return Some("needs a cover image".into());
        }
        None
    }
}

/// One game on its way onto the cartridge.
pub struct Draft {
    pub source: PathBuf,
    pub details: Details,
}

impl Draft {
    fn new(ctx: &egui::Context, source: PathBuf) -> Draft {
        Draft {
            details: Details::new(ctx, &source, None),
            source,
        }
    }

    pub fn blocker(&self) -> Option<String> {
        self.details.blocker(true)
    }

    pub fn bytes(&self) -> u64 {
        self.details
            .scan
            .as_ref()
            .map(|s| s.total_bytes)
            .unwrap_or(0)
    }
}

/// One game already on the cartridge, opened for changes.
///
/// Its files are where they are. The slug never moves — `name` and
/// `games/<slug>/` are linked only at add time, and nothing afterwards reads
/// the name to find the files — so even a rename is a catalog rewrite and
/// nothing else. See `../TODO.md`.
pub struct Edit {
    /// The catalog row as the drive holds it now.
    pub original: Entry,
    /// `<root>/games/<slug>`, where its files already are.
    pub dir: PathBuf,
    pub slug: String,
    /// Whether the row is expanded. Changes outlive being collapsed, so this is
    /// only what is drawn.
    pub open: bool,
    /// The id found beside the exe when the row was opened, so the file is only
    /// rewritten when something actually differs.
    pub appid_original: String,
    pub details: Details,
}

impl Edit {
    /// Opens `entry` for editing, or `None` if its catalog paths don't name a
    /// folder on this cartridge — the same containment check removal makes.
    pub fn start(ctx: &egui::Context, root: &Path, entry: &Entry) -> Option<Edit> {
        let dir = catalog::gameDir(root, entry)?;
        let slug = catalog::slugOf(entry)?;
        let mut details = Details::new(ctx, &dir, Some(catalog::exeRelative(entry)?));
        details.name = entry.name.clone();
        details.steam = entry.steam;
        // Reads the steam_appid.txt already beside the exe, if there is one.
        details.detectAppid(&dir);

        Some(Edit {
            appid_original: details.appid.clone(),
            original: entry.clone(),
            dir,
            slug,
            open: true,
            details,
        })
    }

    /// The catalog row this edit produces.
    pub fn entry(&self) -> Entry {
        Entry {
            name: self.details.name.trim().to_string(),
            exe: self
                .details
                .exeRelative()
                .map(|relative| catalog::exePath(&self.slug, &relative))
                .unwrap_or_else(|| self.original.exe.clone()),
            image: match &self.details.image {
                Some(source) => catalog::imagePath(&self.slug, source),
                None => self.original.image.clone(),
            },
            steam: self.details.steam,
        }
    }

    /// Whether `steam_appid.txt` has to be written again: the box was just
    /// ticked, the id changed, or the exe moved to a different folder.
    pub fn appidRewrite(&self) -> Option<u32> {
        let appid = self.details.appidValue()?;
        let moved = self.entry().exe != self.original.exe;
        let changed = self.details.appid.trim() != self.appid_original.trim();
        (self.details.steam && (!self.original.steam || changed || moved)).then_some(appid)
    }

    /// True when this edit would change anything on the cartridge.
    ///
    /// A replacement cover counts even when it lands on the same path — the
    /// entry is identical, and the file is not.
    pub fn changed(&self) -> bool {
        self.entry() != self.original
            || self.details.image.is_some()
            || self.appidRewrite().is_some()
    }

    pub fn blocker(&self) -> Option<String> {
        self.details.blocker(false)
    }
}

pub struct App {
    pub screen: Screen,
    pub mode: Mode,

    pub volumes: Vec<Volume>,
    pub target: Option<usize>,

    /// What the cartridge should be called — the drive's volume label. Seeded
    /// from the drive's current name when one is picked, so leaving it alone
    /// means leaving the name alone.
    pub name: String,

    /// Edit mode: what is already on the cartridge, and which of it to delete.
    pub existing: Vec<Entry>,
    pub remove: Vec<bool>,
    /// Per-entry changes, `Some` once a row has been opened. Kept the same
    /// length as `existing`.
    pub edits: Vec<Option<Edit>>,

    /// Edit mode: set when the cartridge's launcher states a version other than
    /// `version::bundled()`. `None` covers "not a cartridge", "matches" and
    /// "carries no readable version" alike — none is a reason to offer an
    /// update. Read off the verified signature when the volume was picked, so
    /// there is no pending state and nothing to poll.
    pub staleLauncher: Option<Version>,
    /// The keeper counterpart to `staleLauncher` — also covers a
    /// launcher-carrying cartridge that predates keeper and has none at all.
    pub staleKeeper: Option<KeeperState>,

    pub drafts: Vec<Draft>,
    pub job: Option<Job>,
    pub outcome: Option<Result<Vec<String>, String>>,

    /// The plan held back while its drive is being woken. `Some` only for the
    /// moment the probe runs — see [`App::start`].
    pending: Option<Plan>,

    /// Shown at the top of whatever screen is up, until dismissed.
    pub error: Option<String>,

    // ── Job 2 ────────────────────────────────────────────────────────────
    pub listener_installs: Vec<listener::Installed>,
    pub listener_start_now: bool,
    /// Whether installing should also stop Windows opening a folder when a
    /// drive arrives. Opt-in because it is a user-wide setting — see
    /// [`crate::autoplay`].
    pub suppress_autoplay: bool,
}

impl App {
    pub fn new() -> App {
        App {
            screen: Screen::Home,
            mode: Mode::Create,
            volumes: volume::list(),
            target: None,
            name: String::new(),
            existing: Vec::new(),
            remove: Vec::new(),
            edits: Vec::new(),
            staleLauncher: None,
            staleKeeper: None,
            drafts: Vec::new(),
            job: None,
            outcome: None,
            pending: None,
            error: None,
            listener_installs: listener::find(),
            listener_start_now: true,
            // Ticked unless it has already been done, so the common case is one
            // less decision and a PC that is already set up is not offered a
            // change that would do nothing.
            suppress_autoplay: !autoplay::suppressed(),
        }
    }

    pub fn refreshVolumes(&mut self) {
        let previous = self.volume().map(|v| v.root.clone());
        self.volumes = volume::list();
        self.target = previous.and_then(|root| self.volumes.iter().position(|v| v.root == root));
    }

    pub fn refreshListeners(&mut self) {
        self.listener_installs = listener::find();
    }

    pub fn volume(&self) -> Option<&Volume> {
        self.target.and_then(|i| self.volumes.get(i))
    }

    /// Picks `volume` and routes to edit or create, the only place that
    /// decision is made. Everything it needs is already known from the drive
    /// listing, so nothing here is asynchronous.
    pub fn chooseVolume(&mut self, index: usize) {
        self.target = Some(index);
        self.drafts.clear();
        self.error = None;
        self.staleLauncher = None;
        self.staleKeeper = None;

        let Some(volume) = self.volumes.get(index) else {
            return;
        };

        // The picker doesn't offer a refused drive, so reaching here means the
        // list went stale under a click — a drive unplugged and its letter
        // reused, most plausibly. Checked again rather than trusted, because the
        // thing on the other side of this is a multi-gigabyte write.
        if !volume.allowed() {
            self.error = Some(format!(
                "{} cannot be used: {}",
                volume.root.display(),
                volume.eligibility.reason()
            ));
            self.target = None;
            return;
        }
        let root = volume.root.clone();
        // Seeded from the drive rather than left blank: an empty field would
        // read as "this cartridge has no name" and clear a label the user never
        // meant to touch.
        self.name = volume.label.clone();

        if volume.is_cartridge {
            self.mode = Mode::Edit;
            match catalog::read(&root) {
                Ok(entries) => {
                    self.remove = vec![false; entries.len()];
                    self.edits = entries.iter().map(|_| None).collect();
                    self.existing = entries;
                    // Its signature already told us this when the drive was
                    // listed, so there is nothing to start and nothing to wait
                    // for. Any position differing counts — not just the major,
                    // which is all the listener cares about at runtime. This is
                    // "does the cartridge have the newest launcher this
                    // installer knows how to write", not "will these two
                    // programs still talk to each other".
                    self.staleLauncher = match (
                        volume
                            .launcher_version
                            .as_deref()
                            .and_then(common::version::parse),
                        version::bundled(),
                    ) {
                        (Some(theirs), Some(ours)) if theirs != ours => Some(theirs),
                        _ => None,
                    };
                    // Same question for the keeper, plus one launcher_version
                    // doesn't have to ask: a launcher-carrying cartridge can
                    // simply predate keeper and have no file to attest at all.
                    // Only offered when this build actually carries one —
                    // `bundledKeeper()` is `None` under the payload-optional
                    // escape hatch, and there is nothing to install then.
                    self.staleKeeper = version::bundledKeeper().and_then(|ours| {
                        match volume
                            .keeper_version
                            .as_deref()
                            .and_then(common::version::parse)
                        {
                            Some(theirs) if theirs != ours => Some(KeeperState::Stale(theirs)),
                            Some(_) => None,
                            None => Some(KeeperState::Missing),
                        }
                    });
                }
                Err(e) => {
                    // Refusing here is the point: writing a new catalog over one
                    // we couldn't parse would silently drop games that are on
                    // the cartridge right now.
                    self.error = Some(format!("{e}\n\nFix or delete that file and try again."));
                    self.target = None;
                    return;
                }
            }
            self.screen = Screen::Games;
        } else {
            self.mode = Mode::Create;
            self.existing.clear();
            self.remove.clear();
            self.edits.clear();
            // Straight to the games screen. Creating a cartridge used to stop
            // here to choose a key; there is nothing left to ask.
            self.screen = Screen::Games;
        }
    }

    pub fn addGame(&mut self, ctx: &egui::Context, folder: PathBuf) {
        if self.drafts.iter().any(|d| d.source == folder) {
            self.error = Some(format!(
                "{} is already in this list.",
                folder.file_name().unwrap_or_default().to_string_lossy()
            ));
            return;
        }
        // The other half of the duplicate rule: a folder whose name matches a
        // game already on the cartridge is refused rather than renamed or
        // merged. Renaming produces two entries the user can't tell apart;
        // overwriting destroys an install that may be many gigabytes and may be
        // the only copy. Refusing costs one click — remove the old one first.
        let slug = catalog::slug(&detect::defaultName(&folder));
        if cartridge::takenSlugs(&self.keptEntries()).contains(&slug) {
            self.error = Some(format!(
                "This cartridge already has a game in games/{slug}. \
                 Remove it below first, or rename the folder you are adding."
            ));
            return;
        }
        self.drafts.push(Draft::new(ctx, folder));
    }

    /// Catalog entries that survive this edit, with any changes applied.
    ///
    /// Built in catalog order rather than by appending the changed ones, so a
    /// rename cannot reshuffle the cartridge's game list.
    pub fn keptEntries(&self) -> Vec<Entry> {
        self.existing
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.remove.get(*index).copied().unwrap_or(false))
            .map(|(index, entry)| match self.edits.get(index) {
                Some(Some(edit)) => edit.entry(),
                _ => entry.clone(),
            })
            .collect()
    }

    /// The open edits that would actually change something, with their index.
    fn liveEdits(&self) -> impl Iterator<Item = &Edit> {
        self.edits
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.remove.get(*index).copied().unwrap_or(false))
            .filter_map(|(_, edit)| edit.as_ref())
            .filter(|edit| edit.changed())
    }

    pub fn removedEntries(&self) -> Vec<Entry> {
        self.existing
            .iter()
            .zip(self.remove.iter().chain(std::iter::repeat(&false)))
            .filter(|(_, remove)| **remove)
            .map(|(entry, _)| entry.clone())
            .collect()
    }

    /// Builds the plan, or says what is stopping it.
    pub fn plan(&self) -> Result<Plan, String> {
        let volume = self.volume().ok_or("No volume is selected.")?;

        // The last gate before anything is written, and the reason it is here
        // rather than only in the picker: `plan()` is what the Review screen
        // shows and what the Write button consumes, so a selection that became
        // invalid while the user was adding games — an external drive unplugged,
        // its letter picked up by something internal — cannot get past it.
        if !volume.allowed() {
            return Err(format!(
                "{} cannot be used: {}",
                volume.root.display(),
                volume.eligibility.reason()
            ));
        }
        for draft in &self.drafts {
            if let Some(blocker) = draft.blocker() {
                return Err(format!("{} {blocker}.", draft.details.name));
            }
        }
        // A game on its way off the cartridge is exempt: whatever its row says,
        // nothing is going to be written from it.
        for (index, edit) in self.edits.iter().enumerate() {
            let Some(edit) = edit else { continue };
            if self.remove.get(index).copied().unwrap_or(false) {
                continue;
            }
            if let Some(blocker) = edit.blocker() {
                return Err(format!("{} {blocker}.", edit.original.name));
            }
        }

        // Checked here, where every other blocker is, so a name the drive can't
        // take stops the Review button rather than the last step of a copy that
        // has already run for minutes.
        let name = self.name.trim();
        volume::validateLabel(name, &volume.fs)?;

        let keep = self.keptEntries();
        let mut taken: HashSet<String> = cartridge::takenSlugs(&keep);
        let add = self
            .drafts
            .iter()
            .map(|draft| PlannedGame {
                slug: catalog::uniqueSlug(&draft.details.name, &mut taken),
                source: draft.source.clone(),
                name: draft.details.name.trim().to_string(),
                exeRelative: draft
                    .details
                    .exeRelative()
                    .expect("checked by blocker above"),
                image: draft
                    .details
                    .image
                    .clone()
                    .expect("checked by blocker above"),
                bytes: draft.bytes(),
                steam: draft.details.steam,
                // Gated on the tick: an id left behind by a box the user then
                // unticked must not put a file on the cartridge.
                appid: draft
                    .details
                    .steam
                    .then(|| draft.details.appidValue())
                    .flatten(),
            })
            .collect();

        Ok(Plan {
            root: volume.root.clone(),
            keep,
            remove: self.removedEntries(),
            add,
            edit: self.editedGames(&volume.root),
            label: (name != volume.label).then(|| name.to_string()),
        })
    }

    /// The file-level work each changed game needs, with every path already
    /// resolved — `apply` should not have to work any of it out again.
    ///
    /// A rename produces one of these with nothing in it. That is the point:
    /// its presence is what tells the plan the catalog has to be rewritten, and
    /// what puts the game on the Review screen.
    fn editedGames(&self, root: &Path) -> Vec<EditedGame> {
        self.liveEdits()
            .map(|edit| {
                let entry = edit.entry();
                let mut changes = Vec::new();
                if entry.name != edit.original.name {
                    changes.push(format!("renamed to {}", entry.name));
                }
                if entry.exe != edit.original.exe {
                    changes.push(format!("starts {}", entry.exe));
                }
                if entry.steam != edit.original.steam {
                    changes.push(match entry.steam {
                        true => "needs Steam".into(),
                        false => "no longer needs Steam".into(),
                    });
                }

                // The cover goes wherever its new extension says, which is not
                // always where the old one sat — an older cartridge spells the
                // folder `images/`, and a jpg replacing a png moves it too.
                let cover = edit.details.image.as_ref().map(|source| {
                    changes.push("new cover".into());
                    (
                        source.clone(),
                        root.join(catalog::imagePath(&edit.slug, source)),
                    )
                });
                let stale_cover = cover.as_ref().and_then(|(_, destination)| {
                    catalog::imageFile(root, &edit.original).filter(|old| old != destination)
                });

                let appid = edit.appidRewrite().map(|appid| {
                    if !changes.iter().any(|c| c.starts_with("needs Steam")) {
                        changes.push(format!("Steam app id {appid}"));
                    }
                    let relative = edit
                        .details
                        .exeRelative()
                        .unwrap_or_else(|| PathBuf::from(&entry.exe));
                    (steam::appidFileIn(&edit.dir, &relative), appid)
                });

                EditedGame {
                    name: entry.name,
                    slug: edit.slug.clone(),
                    changes,
                    cover,
                    stale_cover,
                    appid,
                }
            })
            .collect()
    }

    /// Free space, minus what the plan needs — negative when it won't fit.
    pub fn spaceShortfall(&self, plan: &Plan) -> Option<u64> {
        let free = self.volume()?.free_bytes;
        let needed = plan.requiredBytes();
        (needed > free).then(|| needed - free)
    }

    /// Wakes the drive, and starts the write only if it answers.
    ///
    /// Two stages rather than one check inside `cartridge::apply`, because a
    /// drive that does not answer must not land on the Done screen: Done's only
    /// way out is Finish, which resets the wizard and throws away a game list
    /// that took minutes to assemble.
    pub fn start(&mut self, ctx: &egui::Context, plan: Plan) {
        let root = plan.root.clone();
        self.error = None;
        self.pending = Some(plan);
        self.job = Some(
            Job::spawn(ctx, "Waking the cartridge", move |_cancel, report| {
                wake::probe(&root, report).map(|()| Vec::new())
            })
            .uncancellable(),
        );
        self.screen = Screen::Working;
    }

    fn startWrite(&mut self, ctx: &egui::Context, plan: Plan) {
        let games = plan.add.len();
        let removed = plan.remove.len();
        let changed = plan.edit.len();
        let renamed = plan.label.clone();
        self.job = Some(Job::spawn(
            ctx,
            "Writing the cartridge",
            move |cancel, report| {
                cartridge::apply(&plan, cancel, report).map(|warning| {
                    let mut done = Vec::new();
                    if games > 0 {
                        done.push(format!("Copied {games} game(s) onto the cartridge"));
                    }
                    if removed > 0 {
                        done.push(format!("Removed {removed} game(s)"));
                    }
                    if changed > 0 {
                        done.push(format!("Updated {changed} game(s) already there"));
                    }
                    done.push("Wrote launcher.exe and catalog.json".into());
                    // The rename is reported whichever way it went: silently
                    // dropping a name the user typed is the one outcome they would
                    // not find out about until they looked at the drive.
                    match (&renamed, warning) {
                        (_, Some(problem)) => done.push(problem),
                        (Some(name), None) if name.is_empty() => {
                            done.push("Cleared the drive's name".into())
                        }
                        (Some(name), None) => done.push(format!("Named the drive {name}")),
                        (None, None) => {}
                    }
                    done.push("Plug it into a PC running the listener to try it".into());
                    done
                })
            },
        ));
        self.screen = Screen::Working;
    }

    pub fn installListener(&mut self, ctx: &egui::Context) {
        let start_now = self.listener_start_now;
        let suppress_autoplay = self.suppress_autoplay;
        self.startListenerJob(ctx, "Installing the listener", move |_report| {
            listener::install(start_now, suppress_autoplay)
        });
    }

    /// Removes the listener at `dir` — which is the one in
    /// `listener::installDir()`, or a folder an earlier build used.
    pub fn uninstallListener(&mut self, ctx: &egui::Context, dir: PathBuf) {
        self.startListenerJob(ctx, "Removing the listener", move |_report| {
            listener::uninstall(&dir)
        });
    }

    /// Both job-2 operations are over in well under a second but still go
    /// through the worker, so the one Working/Done pair of screens reports every
    /// outcome in the program the same way.
    fn startListenerJob<F>(&mut self, ctx: &egui::Context, title: &str, task: F)
    where
        F: FnOnce(&mut dyn FnMut(cartridge::Progress)) -> Result<Vec<String>, String>
            + Send
            + 'static,
    {
        self.job = Some(
            Job::spawn(ctx, title, move |_cancel, report| {
                report(cartridge::Progress {
                    done: 0,
                    total: 1,
                    label: "Working…".into(),
                });
                task(report)
            })
            .uncancellable(),
        );
        self.screen = Screen::Working;
    }

    /// Rewrites just `launcher.exe` on the current cartridge, independent of
    /// the games plan. `cartridge::apply` also refreshes it, but only as part
    /// of a plan that changes something else — an empty plan cannot reach
    /// Review (`Plan::isEmpty`), so this is the only route for a cartridge
    /// whose games and name are already correct.
    pub fn updateLauncher(&mut self, ctx: &egui::Context) {
        let Some(root) = self.volume().map(|v| v.root.clone()) else {
            return;
        };
        self.startListenerJob(ctx, "Updating the launcher", move |report| {
            // The same gate as a full write, for the same reason: this one goes
            // straight at the drive. A failure here can land on Done, though —
            // there is no game list to lose.
            wake::probe(&root, report)?;
            let bytes = payload::launcher()?;
            copy::bytes(&root.join(crate::constants::LAUNCHER_NAME), &bytes)
                .map_err(|e| e.message())?;
            Ok(vec!["Updated launcher.exe".into()])
        });
    }

    /// Rewrites just `keeper.exe` on the current cartridge — the keeper
    /// counterpart to [`Self::updateLauncher`], for the same reason: an empty
    /// plan cannot reach Review, so a cartridge whose games are already
    /// correct still needs a route to a fresh keeper.
    pub fn updateKeeper(&mut self, ctx: &egui::Context) {
        let Some(root) = self.volume().map(|v| v.root.clone()) else {
            return;
        };
        self.startListenerJob(ctx, "Updating the keeper", move |report| {
            wake::probe(&root, report)?;
            let bytes = payload::keeper()?;
            copy::bytes(&root.join(crate::constants::KEEPER_NAME), &bytes)
                .map_err(|e| e.message())?;
            Ok(vec!["Updated keeper.exe".into()])
        });
    }

    /// Moves a finished job on: the wake-up into the write it was gating,
    /// anything else onto the Done screen.
    pub fn pollJob(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.job else { return };
        job.poll();
        if !job.finished() {
            return;
        }
        let outcome = self.job.take().expect("just checked").outcome;

        if let Some(plan) = self.pending.take() {
            match outcome {
                Some(Ok(_)) => self.startWrite(ctx, plan),
                // Back to Review, not Done. The drafts were never touched and
                // `plan()` rebuilds from them, so plugging the drive back in
                // and pressing Write again is the whole recovery.
                problem => {
                    self.error = Some(match problem {
                        Some(Err(reason)) => reason,
                        _ => "The cartridge could not be woken.".into(),
                    });
                    self.screen = Screen::Review;
                }
            }
            return;
        }

        self.outcome = outcome;
        self.screen = Screen::Done;
        self.refreshVolumes();
        self.refreshListeners();
    }

    /// Back to the start, keeping nothing but the volume list.
    pub fn reset(&mut self) {
        *self = App::new();
    }
}
