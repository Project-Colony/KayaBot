//! Where KayaBot keeps its files: `Colony/KayaBot/` in the platform's config
//! root, from `colony_ui::paths`. Older versions wrote elsewhere, so the first
//! start copies those files over once.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The program name every Colony path is built from.
pub const PROGRAM: &str = "KayaBot";

/// Written into the config directory once the legacy files are in it.
const MIGRATED_MARKER: &str = ".migrated";

/// The directory chosen by [`init`] for this run.
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The config directory: `~/.config/Colony/KayaBot` on Linux,
/// `%LOCALAPPDATA%\Colony\KayaBot` on Windows,
/// `~/Library/Application Support/Colony/KayaBot` on macOS. It is the older
/// location instead when [`init`] could not copy the files out of it.
pub fn config_dir() -> Option<PathBuf> {
    CONFIG_DIR.get().cloned().or_else(default_dir)
}

#[cfg(not(test))]
fn default_dir() -> Option<PathBuf> {
    colony_ui::paths::config_dir(PROGRAM).ok()
}

/// Unit tests save preferences and keys like the program does, so they get a
/// directory of their own and never touch the user's profile.
#[cfg(test)]
fn default_dir() -> Option<PathBuf> {
    let dir = std::env::temp_dir().join(format!("kayabot-unit-tests-{}", std::process::id()));
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Picks the config directory for this run, copying the files of an older
/// location into it first when it is still empty. Call once, at startup,
/// before anything reads a file.
pub fn init() -> Option<PathBuf> {
    let dir = match colony_ui::paths::config_dir(PROGRAM) {
        Ok(dir) => migrate(&dir, &legacy_dirs()),
        Err(err) => {
            eprintln!("No config directory for {PROGRAM}: {err}");
            return None;
        }
    };
    Some(CONFIG_DIR.get_or_init(|| dir).clone())
}

/// Where older versions kept their files, newest first.
fn legacy_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // Windows builds used the roaming profile before the Colony layout.
    if cfg!(windows)
        && let Some(roaming) = dirs::config_dir()
    {
        dirs.push(roaming.join("Colony").join(PROGRAM));
    }
    // The first versions wrote to a dot directory in the home folder.
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".kayabot"));
    }
    dirs
}

/// Copies the files of the first legacy directory that has any into `new_dir`,
/// unless `new_dir` already holds files, and returns the directory to use.
///
/// A legacy directory is never modified, so a failed or wrong copy loses
/// nothing. When the copy fails, whatever it wrote is removed and the legacy
/// directory is used for this run: an empty profile is never the fallback.
fn migrate(new_dir: &Path, legacy_dirs: &[PathBuf]) -> PathBuf {
    if has_entries(new_dir) {
        return new_dir.to_path_buf();
    }
    for legacy in legacy_dirs {
        if legacy == new_dir || !has_entries(legacy) {
            continue;
        }
        return match copy_files(legacy, new_dir) {
            Ok(()) => {
                let marker = new_dir.join(MIGRATED_MARKER);
                if let Err(err) = fs::write(&marker, legacy.display().to_string()) {
                    eprintln!("Failed to write {}: {err}", marker.display());
                }
                new_dir.to_path_buf()
            }
            Err(err) => {
                eprintln!(
                    "Failed to copy {} into {}: {err}. Using the old location.",
                    legacy.display(),
                    new_dir.display()
                );
                legacy.clone()
            }
        };
    }
    new_dir.to_path_buf()
}

fn has_entries(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// Copies every regular file of `from` into `to`. On failure the files copied
/// so far are removed again, so the next start retries from scratch.
/// Sub-directories held caches older versions could rebuild: they are skipped.
fn copy_files(from: &Path, to: &Path) -> io::Result<()> {
    let mut copied = Vec::new();
    let result = (|| {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let target = to.join(entry.file_name());
            // fs::copy keeps the permission bits, so api_keys.toml stays 0600.
            fs::copy(entry.path(), &target)?;
            copied.push(target);
        }
        Ok(())
    })();
    if result.is_err() {
        for path in copied {
            let _ = fs::remove_file(path);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_config_dir_with_files_is_kept_as_is() {
        let root = TempDir::new("paths-kept");
        let new_dir = root.join("new");
        let legacy = root.join("legacy");
        fs::create_dir_all(&new_dir).expect("new dir");
        fs::create_dir_all(&legacy).expect("legacy dir");
        fs::write(new_dir.join("preferences.json"), "{}").expect("new file");
        fs::write(legacy.join("preferences.toml"), "theme = \"mocha\"").expect("legacy file");

        assert_eq!(migrate(&new_dir, std::slice::from_ref(&legacy)), new_dir);
        assert!(!new_dir.join("preferences.toml").exists());
        assert!(!new_dir.join(MIGRATED_MARKER).exists());
    }

    #[test]
    fn legacy_files_are_copied_once_and_left_in_place() {
        let root = TempDir::new("paths-copied");
        let new_dir = root.join("new");
        let empty = root.join("empty");
        let legacy = root.join("legacy");
        fs::create_dir_all(&new_dir).expect("new dir");
        fs::create_dir_all(&empty).expect("empty dir");
        fs::create_dir_all(legacy.join("cache")).expect("legacy dir");
        fs::write(legacy.join("config.toml"), "omdb_api_key = \"k\"").expect("config");
        fs::write(legacy.join("preferences.toml"), "theme = \"mocha\"").expect("prefs");

        let dirs = [root.join("missing"), empty, legacy.clone()];
        assert_eq!(migrate(&new_dir, &dirs), new_dir);
        assert_eq!(
            fs::read_to_string(new_dir.join("preferences.toml")).expect("copied"),
            "theme = \"mocha\""
        );
        assert!(new_dir.join("config.toml").exists());
        assert!(!new_dir.join("cache").exists());
        assert!(new_dir.join(MIGRATED_MARKER).exists());
        // The old location is untouched.
        assert!(legacy.join("config.toml").exists());
        assert!(legacy.join("preferences.toml").exists());

        // The next start finds the new directory populated and stops there.
        fs::write(legacy.join("preferences.toml"), "theme = \"latte\"").expect("rewrite");
        assert_eq!(migrate(&new_dir, &dirs), new_dir);
        assert_eq!(
            fs::read_to_string(new_dir.join("preferences.toml")).expect("kept"),
            "theme = \"mocha\""
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_copy_falls_back_to_the_legacy_dir() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new("paths-failed");
        let new_dir = root.join("new");
        let legacy = root.join("legacy");
        fs::create_dir_all(&new_dir).expect("new dir");
        fs::create_dir_all(&legacy).expect("legacy dir");
        fs::write(legacy.join("preferences.toml"), "theme = \"mocha\"").expect("prefs");
        // A read-only target makes every copy fail, as a full disk would.
        fs::set_permissions(&new_dir, fs::Permissions::from_mode(0o500)).expect("chmod");
        if fs::write(new_dir.join("probe"), "").is_ok() {
            // Running as root: permissions do not stop the copy.
            return;
        }

        let chosen = migrate(&new_dir, std::slice::from_ref(&legacy));

        fs::set_permissions(&new_dir, fs::Permissions::from_mode(0o700)).expect("chmod back");
        assert_eq!(chosen, legacy);
        assert!(!has_entries(&new_dir));
        assert!(legacy.join("preferences.toml").exists());
    }
}
