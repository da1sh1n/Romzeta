// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Every test in this crate, one submodule per source module. Run with
//! `cargo test -p installer` — this crate is not in the workspace's
//! `default-members`, so a bare `cargo test` skips it.

// ########## INSTALLER TESTS ##########

mod payload {
    use crate::payload::{LAUNCHER_BYTES, LISTENER_BYTES, launcher, listener};

    /// The launcher is carried compressed and written uncompressed, and the
    /// minisign signature riding inside it *is* the cartridge's identity. A
    /// single byte off and the cartridge still looks perfect, still contains a
    /// launcher of the right name and size, and is silently ignored by every
    /// listener — with no symptom but nothing happening.
    ///
    /// So this checks the thing that cannot be checked by looking at the drive:
    /// that what comes out of the payload is still signed.
    #[test]
    fn what_unpacks_is_still_signed() {
        for (name, unpacked, expected) in [
            ("launcher.exe", launcher(), LAUNCHER_BYTES),
            ("listener.exe", listener(), LISTENER_BYTES),
        ] {
            let bytes = unpacked.unwrap_or_else(|e| panic!("{name} did not unpack: {e}"));
            assert_eq!(
                bytes.len() as u64,
                expected,
                "{name} unpacked to the wrong size"
            );
            assert!(
                sigblock::isSigned(&bytes),
                "{name} came out of the payload without its signature — every cartridge \
                 this installer writes would be ignored by every listener"
            );
        }
    }
}

mod font {
    use crate::font::{FALLBACK, SYSTEM, definitions};
    use egui::FontFamily;

    /// epaint panics — `FontFamily::… is not bound to any fonts` — the first time
    /// a family with nothing behind it is used, and it does that lazily. Nothing
    /// in this program asks for monospace today, so the crash would arrive the
    /// day something did.
    #[test]
    fn every_family_has_something_behind_it() {
        let fonts = definitions();
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let chain = fonts
                .families
                .get(&family)
                .unwrap_or_else(|| panic!("{family:?} was not bound at all"));
            assert!(!chain.is_empty(), "{family:?} was bound to an empty list");
            for name in chain {
                assert!(
                    fonts.font_data.contains_key(name),
                    "{family:?} names {name:?}, which has no font data"
                );
            }
        }
    }

    /// The fallback is last, so a glyph the system font lacks still draws.
    #[test]
    fn the_fallback_is_always_there_and_always_last() {
        let fonts = definitions();
        for chain in fonts.families.values() {
            assert_eq!(chain.last().map(String::as_str), Some(FALLBACK));
        }
    }

    /// The one thing looking at the window cannot tell you. If the face lookup,
    /// the registry walk or the file read breaks, the wizard still comes up —
    /// drawn in Ubuntu-Light, which is not what this machine uses anywhere else.
    ///
    /// Like [`super::volume`]'s tests, this one asserts against the real machine.
    #[test]
    #[cfg(windows)]
    fn the_system_font_is_the_one_actually_used() {
        let fonts = definitions();
        let chain = &fonts.families[&FontFamily::Proportional];
        assert_eq!(
            chain.first().map(String::as_str),
            Some(SYSTEM),
            "fell back to {FALLBACK}: this Windows' UI font could not be found or read, \
             so the wizard would draw in a typeface nothing else on the desktop uses"
        );
    }
}

mod catalog {
    use crate::catalog::{Entry, gameDir, imageFile, imagePath, slug};
    use std::path::Path;

    #[test]
    fn the_steam_flag_is_written_only_when_it_is_set() {
        // Both halves are the contract with the launcher's `Game`. Writing
        // `"steam": false` into every entry would add a key to catalogs that
        // never needed one, and failing to read a catalog that lacks it would
        // break every cartridge written before the checkbox existed.
        let plain = Entry {
            name: "celeste".into(),
            exe: "games/celeste/celeste.exe".into(),
            image: "assets/images/celeste.png".into(),
            steam: false,
        };
        let json = serde_json::to_string(&plain).unwrap();
        assert!(!json.contains("steam"), "{json}");
        assert!(
            serde_json::to_string(&Entry {
                steam: true,
                ..plain.clone()
            })
            .unwrap()
            .contains(r#""steam":true"#)
        );

        let older = r#"[{"name":"celeste","exe":"games/celeste/celeste.exe",
                        "image":"assets/images/celeste.png"}]"#;
        let read: Vec<Entry> = serde_json::from_str(older).unwrap();
        assert_eq!(read, vec![plain]);
    }

    #[test]
    fn covers_are_written_under_assets() {
        // The path this produces goes into catalog.json and is what the
        // launcher asks its app:// protocol for, so the prefix is a contract
        // between the two crates rather than a detail of this one.
        assert_eq!(
            imagePath("bg3", Path::new(r"C:\art\cover.png")),
            "assets/images/bg3.png"
        );
        // The source extension is kept: the webview goes by content, not name,
        // and renaming a jpg to png only makes the cartridge harder to read.
        assert_eq!(
            imagePath("celeste", Path::new(r"C:\art\cover.JPG")),
            "assets/images/celeste.jpg"
        );
    }

    #[test]
    fn a_cover_written_by_an_older_installer_still_resolves() {
        // Cartridges made before covers moved under assets/ say `images/...`
        // in their catalog. Nothing migrates them — the launcher serves both
        // prefixes — so removal has to keep finding the file too, or editing
        // an old cartridge would leave its art behind.
        let root = Path::new(r"E:\");
        let legacy = Entry {
            name: "bg3".into(),
            exe: "games/bg3/bg3.exe".into(),
            image: "images/bg3.png".into(),
            steam: false,
        };
        assert_eq!(
            imageFile(root, &legacy),
            Some(root.join("images").join("bg3.png"))
        );

        let current = Entry {
            image: "assets/images/bg3.png".into(),
            ..legacy
        };
        assert_eq!(
            imageFile(root, &current),
            Some(root.join("assets").join("images").join("bg3.png"))
        );
    }

    #[test]
    fn an_entry_says_which_folder_and_exe_are_its_own() {
        use crate::catalog::{exeRelative, slugOf};
        use std::path::PathBuf;

        let nested = Entry {
            name: "Portal 2".into(),
            exe: "games/portal_2/bin/portal2.exe".into(),
            image: "assets/images/portal_2.png".into(),
            steam: true,
        };
        assert_eq!(slugOf(&nested).as_deref(), Some("portal_2"));
        assert_eq!(exeRelative(&nested), Some(PathBuf::from("bin/portal2.exe")));

        // The slug is read off the path, never re-derived from the name — the
        // two part company the moment a game is renamed, and the folder is what
        // the files are actually in.
        let renamed = Entry {
            name: "Something Else Entirely".into(),
            ..nested.clone()
        };
        assert_eq!(slugOf(&renamed).as_deref(), Some("portal_2"));

        // Anything not under games/<slug>/<file> names no folder of its own,
        // which is the same refusal `gameDir` makes.
        for exe in ["games/loose.exe", "elsewhere/x/y.exe", "games/portal_2"] {
            let odd = Entry {
                exe: exe.into(),
                ..nested.clone()
            };
            assert_eq!(exeRelative(&odd), None, "{exe}");
        }
    }

    #[test]
    fn slugs_are_safe_on_any_filesystem() {
        // A game's name becomes a folder name, so this is the point where a
        // name someone typed turns into a path.
        assert_eq!(slug("Baldur's Gate 3"), "baldur_s_gate_3");
        assert_eq!(slug("Hollow Knight"), "hollow_knight");
        assert_eq!(slug("  NieR:Automata™  "), "nier_automata");
        assert_eq!(slug("!!!"), "game");
        assert_eq!(slug(""), "game");
    }

    #[test]
    fn removal_paths_stay_inside_the_cartridge() {
        let root = Path::new(r"E:\");
        let escape = Entry {
            name: "evil".into(),
            exe: "../../Windows/System32/cmd.exe".into(),
            image: "../../Windows/x.png".into(),
            steam: false,
        };
        assert_eq!(gameDir(root, &escape), None);
        assert_eq!(imageFile(root, &escape), None);

        let ok = Entry {
            name: "bg3".into(),
            exe: "games/bg3/bin/bg3.exe".into(),
            image: "images/bg3.png".into(),
            steam: false,
        };
        assert_eq!(gameDir(root, &ok), Some(root.join("games").join("bg3")));
        assert_eq!(
            imageFile(root, &ok),
            Some(root.join("images").join("bg3.png"))
        );

        // An exe sitting directly in games/ names no folder to delete.
        let shallow = Entry {
            name: "loose".into(),
            exe: "games/loose.exe".into(),
            image: "images/loose.png".into(),
            steam: false,
        };
        assert_eq!(gameDir(root, &shallow), None);
    }
}

mod steam {
    use crate::app::Details;
    use crate::detect::Scan;
    use crate::steam::{appidFileIn, manifest, parse};
    use std::path::{Path, PathBuf};

    /// A game past every earlier blocker, so what the app id does to `blocker`
    /// is the only thing under test.
    fn ready(appid: &str, steam: bool) -> Details {
        Details {
            name: "Portal 2".into(),
            scanning: None,
            scan: Some(Scan {
                candidates: Vec::new(),
                total_bytes: 0,
                file_count: 0,
                cancelled: false,
            }),
            selected: None,
            manual_exe: Some(PathBuf::from("portal2.exe")),
            exe_fallback: None,
            image: Some(PathBuf::from(r"C:\art\cover.png")),
            image_warning: None,
            steam,
            appid: appid.into(),
            appid_found: None,
        }
    }

    #[test]
    fn an_app_id_is_digits_and_not_zero() {
        assert_eq!(parse("620"), Some(620));
        // Whatever a file or a paste brought along with it.
        assert_eq!(parse(" 620\r\n"), Some(620));
        assert_eq!(parse(""), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse("620a"), None);
        // What an empty or malformed manifest would otherwise parse to.
        assert_eq!(parse("0"), None);
    }

    #[test]
    fn a_manifest_gives_up_its_id_and_folder() {
        let acf = "\"AppState\"\n\
                   {\n\
                   \t\"appid\"\t\t\"620\"\n\
                   \t\"universe\"\t\t\"1\"\n\
                   \t\"name\"\t\t\"Portal 2\"\n\
                   \t\"installdir\"\t\t\"Portal 2\"\n\
                   }\n";
        assert_eq!(manifest(acf), Some((620, "Portal 2".into())));

        // Half a manifest is no answer at all — the folder is what says this
        // manifest is the right one out of a library holding fifty.
        assert_eq!(manifest("\t\"appid\"\t\t\"620\"\n"), None);
        assert_eq!(manifest(""), None);
    }

    #[test]
    fn the_id_file_lands_beside_the_exe() {
        // The rule that matters: steam_api.dll reads the file next to the
        // module it is loaded into, so a nested exe does not get one at the
        // game folder's root.
        let destination = Path::new(r"E:\games\portal2");
        assert_eq!(
            appidFileIn(destination, Path::new("bin/portal2.exe")),
            destination.join("bin").join("steam_appid.txt")
        );
        assert_eq!(
            appidFileIn(destination, Path::new("portal2.exe")),
            destination.join("steam_appid.txt")
        );
    }

    #[test]
    fn a_ticked_steam_box_needs_an_id() {
        assert_eq!(
            ready("", true).blocker(true).as_deref(),
            Some("needs a Steam app id")
        );
        assert_eq!(
            ready("not a number", true).blocker(true).as_deref(),
            Some("has a Steam app id that isn't a number")
        );
        assert_eq!(ready("620", true).blocker(true), None);
        // Unticked, the field is nobody's business either way.
        assert_eq!(ready("", false).blocker(true), None);
        assert_eq!(ready("nonsense", false).blocker(true), None);
    }
}

mod edit {
    use crate::app::{Details, Edit};
    use crate::catalog::Entry;
    use crate::detect::Scan;
    use std::path::PathBuf;

    fn onCartridge() -> Edit {
        let original = Entry {
            name: "Portal 2".into(),
            exe: "games/portal_2/bin/portal2.exe".into(),
            image: "assets/images/portal_2.png".into(),
            steam: true,
        };
        Edit {
            dir: PathBuf::from(r"E:\games\portal_2"),
            slug: "portal_2".into(),
            open: true,
            appid_original: "620".into(),
            details: Details {
                name: original.name.clone(),
                scanning: None,
                scan: Some(Scan {
                    candidates: Vec::new(),
                    total_bytes: 0,
                    file_count: 0,
                    cancelled: false,
                }),
                selected: None,
                manual_exe: None,
                exe_fallback: Some(PathBuf::from("bin/portal2.exe")),
                image: None,
                image_warning: None,
                steam: original.steam,
                appid: "620".into(),
                appid_found: None,
            },
            original,
        }
    }

    #[test]
    fn an_untouched_row_changes_nothing() {
        let edit = onCartridge();
        assert!(!edit.changed());
        assert_eq!(edit.entry(), edit.original);
        // Nothing to fall back *from*: the exe the catalog names is what the
        // picker reports until the user picks another.
        assert_eq!(
            edit.details.exeRelative(),
            Some(PathBuf::from("bin/portal2.exe"))
        );
    }

    #[test]
    fn a_rename_leaves_every_path_where_it_was() {
        // The whole reason renaming is cheap. Re-slugging the new name would
        // mean moving every file in a folder that can be tens of gigabytes,
        // for a path nobody ever sees.
        let mut edit = onCartridge();
        edit.details.name = "  Portal II  ".into();

        let entry = edit.entry();
        assert!(edit.changed());
        assert_eq!(entry.name, "Portal II");
        assert_eq!(entry.exe, edit.original.exe);
        assert_eq!(entry.image, edit.original.image);
        // A rename touches no file, so nothing to rewrite beside the exe.
        assert_eq!(edit.appidRewrite(), None);
    }

    #[test]
    fn a_new_cover_follows_its_own_extension() {
        let mut edit = onCartridge();
        edit.details.image = Some(PathBuf::from(r"C:\art\new.JPG"));
        assert_eq!(edit.entry().image, "assets/images/portal_2.jpg");

        // Same extension means the same path — the entry is untouched and it
        // is the file underneath that changes, which `changed` has to catch on
        // its own or the new art would never be copied.
        let mut same = onCartridge();
        same.details.image = Some(PathBuf::from(r"C:\art\new.png"));
        assert_eq!(same.entry(), same.original);
        assert!(same.changed());
    }

    #[test]
    fn the_app_id_file_is_rewritten_only_when_it_would_differ() {
        // Already ticked, same id, same exe: nothing to write.
        assert_eq!(onCartridge().appidRewrite(), None);

        let mut retyped = onCartridge();
        retyped.details.appid = "400".into();
        assert_eq!(retyped.appidRewrite(), Some(400));

        // Just ticked: the file may not be there at all yet.
        let mut ticked = onCartridge();
        ticked.original.steam = false;
        assert_eq!(ticked.appidRewrite(), Some(620));

        // The exe moved, so the file has to appear beside the new one — the
        // old copy is left alone, being one we cannot tell from the game's.
        let mut moved = onCartridge();
        moved.details.manual_exe = Some(PathBuf::from("portal2.exe"));
        assert_eq!(moved.appidRewrite(), Some(620));

        // Unticked: the launcher stops starting Steam and no file is touched.
        let mut unticked = onCartridge();
        unticked.details.steam = false;
        assert_eq!(unticked.appidRewrite(), None);
        assert!(unticked.changed());
        assert!(!unticked.entry().steam);
    }
}

mod image {
    use crate::image::parse;

    fn pngHeader(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    fn riff(chunk: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(b"WEBP");
        bytes.extend_from_slice(chunk);
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    #[test]
    fn reads_a_png() {
        assert_eq!(parse(&pngHeader(600, 900)), Some((600, 900)));
    }

    #[test]
    fn reads_an_animated_webp() {
        // VP8X — the shape the covers in this project actually are, and the
        // reason the format is sniffed from bytes rather than trusted from the
        // `.png` name they are usually saved under.
        let mut body = vec![0x10, 0, 0, 0]; // flags: has animation
        body.extend_from_slice(&[0x57, 0x02, 0x00]); // 600-1, 24-bit LE
        body.extend_from_slice(&[0x83, 0x03, 0x00]); // 900-1
        assert_eq!(parse(&riff(b"VP8X", &body)), Some((600, 900)));
    }

    #[test]
    fn reads_a_lossless_webp() {
        let bits: u32 = (599) | (899 << 14);
        let mut body = vec![0x2f];
        body.extend_from_slice(&bits.to_le_bytes());
        body.extend_from_slice(&[0; 8]);
        assert_eq!(parse(&riff(b"VP8L", &body)), Some((600, 900)));
    }

    #[test]
    fn reads_a_lossy_webp() {
        let mut body = vec![0x00, 0x00, 0x00, 0x9d, 0x01, 0x2a];
        body.extend_from_slice(&600u16.to_le_bytes());
        body.extend_from_slice(&900u16.to_le_bytes());
        body.extend_from_slice(&[0; 8]);
        assert_eq!(parse(&riff(b"VP8 ", &body)), Some((600, 900)));
    }

    #[test]
    fn reads_a_gif() {
        let mut bytes = b"GIF89a".to_vec();
        bytes.extend_from_slice(&600u16.to_le_bytes());
        bytes.extend_from_slice(&900u16.to_le_bytes());
        assert_eq!(parse(&bytes), Some((600, 900)));
    }

    #[test]
    fn reads_a_jpeg_behind_a_metadata_segment() {
        let mut bytes = vec![0xff, 0xd8];
        // An APP1 (EXIF) segment the size walk has to step over.
        bytes.extend_from_slice(&[0xff, 0xe1, 0x00, 0x10]);
        bytes.extend_from_slice(&[0u8; 14]);
        // SOF0: length, precision, height, width.
        bytes.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08]);
        bytes.extend_from_slice(&900u16.to_be_bytes());
        bytes.extend_from_slice(&600u16.to_be_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        assert_eq!(parse(&bytes), Some((600, 900)));
    }

    #[test]
    fn an_unrecognised_file_is_not_a_failure() {
        assert_eq!(parse(b"this is not a picture at all"), None);
        assert_eq!(parse(&[]), None);
    }
}

mod version {
    #[test]
    fn our_version_is_a_bare_three_part_number() {
        // The same shape the launcher and listener print. Nothing parses the
        // installer's, but three programs answering one question three ways is
        // how the one that *is* parsed eventually drifts.
        let version = env!("CARGO_PKG_VERSION");
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "{version}");
        for part in parts {
            assert!(
                part.parse::<u64>().is_ok(),
                "{version} has a non-numeric part"
            );
        }
    }
}

mod volume {
    /// The build carries at least one usable anchor. Same assertion as the
    /// listener's `this_build_has_something_to_trust` — the two must agree, or a
    /// cartridge one of them would accept the other silently would not.
    #[test]
    fn this_build_has_something_to_trust() {
        use crate::volume::ANCHORS;

        assert!(!ANCHORS.is_empty(), "build.rs produced no trust anchors");
        for anchor in ANCHORS {
            assert!(
                anchor.isUsable(),
                "keys/{}.pub is not a usable minisign public key",
                anchor.name
            );
        }
    }

    /// The finding this change exists to fix: a `launcher.exe` at a drive root is
    /// not believed just because it has the right name. Without a signing key
    /// this cannot construct something that *does* verify — that round trip is
    /// `trust`'s own suite — but every one of these must come back `None`
    /// rather than "close enough".
    #[test]
    fn only_a_verified_signature_makes_a_cartridge() {
        use crate::volume::attestedLauncher;

        let dir =
            std::env::temp_dir().join(format!("romzeta-installer-attest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        // No file at all.
        assert_eq!(attestedLauncher(&dir), None);

        // A file with the right name and nothing else — what running it used to
        // accept.
        std::fs::write(dir.join(crate::cartridge::LAUNCHER_NAME), b"MZ not signed").expect("write");
        assert_eq!(attestedLauncher(&dir), None);

        // A well-formed signature block from a key this build does not carry.
        let signature = "untrusted comment: signature from a key we do not have\n\
                         RUQAAAAAAAAAAOaGxHqZQ0KtvVCJ6iKzXG8bFvKZ0V0kZ1qWzKz0hVYQ4rZ8Xk1t\n\
                         trusted comment: romzeta-launcher 9.9.9 2026-07-30\n\
                         AAAA==\n";
        let signed = sigblock::attach(b"MZ signed by someone else", signature);
        std::fs::write(dir.join(crate::cartridge::LAUNCHER_NAME), signed).expect("write");
        assert_eq!(attestedLauncher(&dir), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn the_drive_windows_is_on_is_refused() {
        use crate::volume::{Eligibility, driveLetter, isSystemDrive, list};
        use std::path::Path;

        // The most important behaviour in this module, asserted against the
        // machine running the test rather than a fixture.
        let system = std::env::var_os("SystemRoot").expect("Windows sets SystemRoot");
        let letter = driveLetter(Path::new(&system)).expect("a drive letter");

        assert!(isSystemDrive(Path::new(&format!("{letter}:\\"))));
        assert!(isSystemDrive(Path::new(&format!(
            "{}:\\",
            letter.to_ascii_lowercase()
        ))));
        assert!(isSystemDrive(Path::new(&format!("{letter}:\\games"))));

        for volume in list() {
            if driveLetter(&volume.root) == Some(letter) {
                assert_eq!(
                    volume.eligibility,
                    Eligibility::SystemDrive,
                    "{} must never be offered",
                    volume.root.display()
                );
                assert!(!volume.allowed());
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn nothing_internal_is_ever_allowed() {
        use crate::volume::{isSystemDrive, list};

        for volume in list() {
            if volume.allowed() {
                assert!(
                    !isSystemDrive(&volume.root),
                    "{} is the system drive",
                    volume.root.display()
                );
                assert!(
                    matches!(volume.bus, "USB" | "FireWire" | "SD" | "MMC" | "removable"),
                    "{} was allowed on a {} bus",
                    volume.root.display(),
                    volume.bus
                );
            }
        }
    }
}

mod wake {
    use crate::wake::{PROBES, probe};
    use std::path::PathBuf;

    fn tempDir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "romzeta-installer-wake-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    /// Three reads and not one. The first can be answered from the OS cache
    /// while the disk underneath is still spinning up, so a probe count of one
    /// would pass on a drive that is not actually awake — which is the whole
    /// thing this gate exists to catch.
    #[test]
    fn a_drive_that_answers_is_read_three_times() {
        let dir = tempDir("awake");
        let mut reports = 0u64;
        let mut last_done = 0u64;
        probe(&dir, &mut |progress| {
            reports += 1;
            last_done = progress.done;
            assert_eq!(progress.total, PROBES);
        })
        .expect("a directory that exists must answer");

        assert_eq!(reports, PROBES + 1, "one report per round, plus the finish");
        assert_eq!(last_done, PROBES, "the bar must end full");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The unplugged drive, and the reason the failure has to name the path:
    /// the message is the only thing the user gets back on the Review screen,
    /// and "it did not work" would leave them nothing to act on.
    #[test]
    fn a_drive_that_is_not_there_fails_on_the_first_round() {
        let dir = tempDir("gone");
        std::fs::remove_dir_all(&dir).expect("remove");

        let problem = probe(&dir, &mut |_| {}).expect_err("a missing path cannot answer");
        assert!(problem.contains(&dir.display().to_string()), "{problem}");
        assert!(
            problem.contains(&format!("probe 1 of {PROBES}")),
            "{problem}"
        );
    }

    /// A drive letter picked up by something that is not a volume root. It
    /// answers `metadata` perfectly well, which is why the kind is checked and
    /// not just the existence.
    #[test]
    fn a_path_that_is_not_a_directory_is_refused() {
        let dir = tempDir("file");
        let file = dir.join("not-a-drive-root");
        std::fs::write(&file, b"").expect("write");

        let problem = probe(&file, &mut |_| {}).expect_err("a file is not a cartridge");
        assert!(problem.contains("not a directory"), "{problem}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
