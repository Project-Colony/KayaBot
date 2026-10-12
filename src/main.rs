// Release builds are GUI programs on Windows: no console window opens next to
// the main one. Debug builds keep the console for eprintln! diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kayabot::config::Config;
use kayabot::{api_keys, paths, ui};

fn main() -> iced::Result {
    // Answered before any window or file code: the release workflow smoke-tests
    // every build with `--version`, and it must exit 0 on a headless runner.
    if wants_version(std::env::args_os()) {
        println!("kayabot {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(dir) = paths::init()
        && let Err(err) = api_keys::migrate_config_credentials(&dir)
    {
        eprintln!("Failed to move API keys out of config.toml: {err}");
    }

    ui::run(Config::load())
}

/// True when the command line asks for the version: `--version` or `-V` as
/// the first argument. Read as `OsString`s so a non-UTF-8 argument cannot
/// panic the GUI start.
fn wants_version(mut args: impl Iterator<Item = std::ffi::OsString>) -> bool {
    args.nth(1)
        .is_some_and(|arg| arg == "--version" || arg == "-V")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wants_version_only_for_a_leading_version_flag() {
        let argv = |args: &[&str]| {
            args.iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>()
                .into_iter()
        };
        assert!(wants_version(argv(&["kayabot", "--version"])));
        assert!(wants_version(argv(&["kayabot", "-V"])));
        assert!(!wants_version(argv(&[])));
        assert!(!wants_version(argv(&["kayabot"])));
        assert!(!wants_version(argv(&["kayabot", "-v"])));
        assert!(!wants_version(argv(&["kayabot", "--versions"])));
        assert!(!wants_version(argv(&["kayabot", "movie.mkv", "--version"])));
    }
}
