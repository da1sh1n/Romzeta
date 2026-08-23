// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Where the signing key lives, how it is read, and where it must never be.

#![allow(non_snake_case)] // camelCase functions

// ########## SIGNING KEYS ##########

mod common;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use common::{Scratch, checks, runTest, verdict};
use xtask::keys::{base64Line, dotenv, keygen, refuseInsideRepo, secretKey};

/// The `.env` a fixture root produces, for the parser tests.
fn envFrom(case: &str, text: &str) -> HashMap<String, String> {
    let dir = Scratch::new(case);
    fs::write(dir.join(".env"), text).expect("write .env");
    dotenv(dir.path())
}

/// A fake repo root whose `.env` points the signing key somewhere outside it,
/// which is the arrangement `secretKey` and `keygen` both expect.
fn rootedAt(case: &str, key: &Path, extra: &str) -> Scratch {
    let root = Scratch::new(case);
    fs::write(
        root.join(".env"),
        format!("ROMZETA_SIGNING_KEY={}\n{extra}", key.display()),
    )
    .expect("write .env");
    root
}

#[test]
fn reads_the_key_out_of_a_pub_file() {
    runTest(|| {
        let text = "untrusted comment: minisign public key A1B2\nRWQf6LRCGA9i53==\n";
        verdict(
            base64Line(text).unwrap_or_else(|| "none".to_owned()),
            "RWQf6LRCGA9i53==",
        )
    });
}

#[test]
fn survives_a_pub_file_someone_edited() {
    runTest(|| {
        // No comment line at all, and trailing blank lines.
        let mut proved = checks();
        proved.expect(
            base64Line("RWQf6LRCGA9i53==\n\n").as_deref() == Some("RWQf6LRCGA9i53=="),
            "a key with no comment still reads",
        );
        proved.expect(
            base64Line("untrusted comment: only a comment\n").is_none(),
            "a comment on its own is not a key",
        );
        proved.expect(base64Line("").is_none(), "nor is an empty file");
        proved.verdict()
    });
}

#[test]
fn parses_the_env_shapes_that_actually_occur() {
    runTest(|| {
        let env = envFrom(
            "xtask-dotenv",
            "# a comment\n\
             \n\
             ROMZETA_SIGNING_KEY=C:\\keys\\romzeta.key\n\
             ROMZETA_SIGNING_PASSWORD = \"hunter2\"\n\
             QUOTED='single'\n",
        );
        let mut proved = checks();
        proved.expect(
            env.get("ROMZETA_SIGNING_KEY").map(String::as_str) == Some("C:\\keys\\romzeta.key"),
            "a bare value keeps its backslashes",
        );
        proved.expect(
            env.get("ROMZETA_SIGNING_PASSWORD").map(String::as_str) == Some("hunter2"),
            "spaces and double quotes are stripped",
        );
        proved.expect(
            env.get("QUOTED").map(String::as_str) == Some("single"),
            "single quotes are stripped too",
        );
        proved.expect(env.len() == 3, "the comment and blank line are skipped");
        proved.verdict()
    });
}

#[test]
fn a_missing_env_file_is_not_an_error() {
    runTest(|| {
        let empty = dotenv(Path::new("/nowhere/at/all"));
        verdict(format!("{} entries", empty.len()), "0 entries")
    });
}

#[test]
fn an_unencrypted_key_loads_with_no_password_at_all() {
    runTest(|| {
        // The regression: minisign's `into_secret_key(None)` does not mean "no
        // password", it means "decrypt, asking if you must" — and it *rejects*
        // a key with no KDF. Routing the default key through it turned "needs
        // nothing" into a prompt claiming the key was password-protected.
        let dir = Scratch::new("xtask-plainkey");
        let key_path = dir.join("romzeta.key");

        let pair = minisign::KeyPair::generate_unencrypted_keypair().expect("keypair");
        fs::write(&key_path, pair.sk.to_box(None).expect("box").to_string()).expect("write key");

        let root = rootedAt("xtask-plainkey-root", &key_path, "");
        let loaded = secretKey(root.path()).unwrap_or_else(|e| panic!("{e}"));
        verdict(
            if loaded == pair.sk {
                "the key we wrote"
            } else {
                "a different key"
            },
            "the key we wrote",
        )
    });
}

#[test]
fn keygen_never_writes_the_password_into_the_key_file() {
    runTest(|| {
        // The other half of the same confusion: `SecretKey::to_box` takes the
        // untrusted *comment*, not a password. Passing the password there put it
        // in cleartext on the line directly above the key it protects.
        const PASSWORD: &str = "correct-horse-battery-staple";
        let dir = Scratch::new("xtask-encryptedkey");
        let key_path = dir.join("romzeta.key");

        let root = rootedAt(
            "xtask-encryptedkey-root",
            &key_path,
            &format!("ROMZETA_SIGNING_PASSWORD={PASSWORD}\n"),
        );
        keygen(root.path(), false).unwrap_or_else(|e| panic!("{e}"));
        let written = fs::read_to_string(&key_path).expect("read key");

        let mut proved = checks();
        proved.expect(
            !written.contains(PASSWORD),
            "the password stayed out of the key file",
        );
        // And it still round-trips, using that password from the same `.env`.
        proved.expect(
            secretKey(root.path()).is_ok(),
            "the key still loads with it",
        );
        proved.verdict()
    });
}

#[test]
fn a_secret_key_inside_the_repo_is_refused() {
    runTest(|| {
        // The mistake this exists to stop: putting the key next to its public
        // half, where the next `git add -A` picks it up.
        let root = Scratch::new("xtask-repo");
        fs::create_dir_all(root.join("keys")).expect("temp repo");
        let elsewhere = Scratch::new("xtask-elsewhere");

        let mut proved = checks();
        proved.expect(
            refuseInsideRepo(root.path(), &root.join("keys").join("romzeta.key")).is_err(),
            "a key inside the repo is refused",
        );
        proved.expect(
            refuseInsideRepo(root.path(), &elsewhere.join("romzeta.key")).is_ok(),
            "a key outside it is allowed",
        );
        proved.verdict()
    });
}
