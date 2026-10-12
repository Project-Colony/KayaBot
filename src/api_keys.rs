//! Credentials for the metadata sources: read from the environment first, then
//! from `api_keys.toml` in the config directory, which only its owner can read.

use std::path::{Path, PathBuf};
use std::{env, fs, io};

use crate::paths;

/// The credentials every provider is built with. A missing key is an empty
/// string: the provider then reports that it is not configured.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub tmdb_token: String,
    pub tvdb_api_key: String,
    pub omdb_api_key: String,
    pub tvmaze_user_agent: String,
}

impl ApiConfig {
    /// Resolves every credential: a `KAYABOT_*` environment variable wins over
    /// the matching `api_keys.toml` entry.
    pub fn load() -> (Self, ApiKeysLoadStatus) {
        let (api_keys, status) = ApiKeysFile::load();
        (Self::resolve(api_keys.as_ref()), status)
    }

    pub fn resolve(api_keys: Option<&ApiKeysFile>) -> Self {
        let file = |pick: fn(&ApiKeysFile) -> &Option<String>| {
            api_keys.and_then(|keys| pick(keys).clone())
        };
        let tmdb_token = env_value("KAYABOT_TMDB_BEARER_TOKEN")
            .or_else(|| env_value("KAYABOT_TMDB_API_KEY"))
            .or_else(|| file(|keys| &keys.tmdb_bearer_token))
            .or_else(|| file(|keys| &keys.tmdb_api_key))
            .unwrap_or_default();
        let tvdb_api_key = env_value("KAYABOT_TVDB_API_KEY")
            .or_else(|| file(|keys| &keys.tvdb_api_key))
            .unwrap_or_default();
        let omdb_api_key = env_value("KAYABOT_OMDB_API_KEY")
            .or_else(|| file(|keys| &keys.omdb_api_key))
            .unwrap_or_default();
        let tvmaze_user_agent = env_value("KAYABOT_TVMAZE_USER_AGENT")
            .or_else(|| env_value("KAYABOT_TVMAZE_API_KEY"))
            .or_else(|| file(|keys| &keys.tvmaze_user_agent))
            .unwrap_or_else(|| "KayaBot".to_string());

        Self {
            tmdb_token,
            tvdb_api_key,
            omdb_api_key,
            tvmaze_user_agent,
        }
    }

    pub fn tmdb_configured(&self) -> bool {
        !self.tmdb_token.trim().is_empty()
    }

    pub fn tvdb_configured(&self) -> bool {
        !self.tvdb_api_key.trim().is_empty()
    }

    pub fn omdb_configured(&self) -> bool {
        !self.omdb_api_key.trim().is_empty()
    }
}

fn env_value(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

/// Whether `api_keys.toml` could be read, and where it was looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeysLoadState {
    Loaded,
    Missing,
    Unreadable,
}

#[derive(Debug, Clone)]
pub struct ApiKeysLoadStatus {
    pub path: Option<PathBuf>,
    pub state: ApiKeysLoadState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ApiKeysFile {
    pub tmdb_bearer_token: Option<String>,
    pub tmdb_api_key: Option<String>,
    pub tvdb_api_key: Option<String>,
    pub omdb_api_key: Option<String>,
    pub tvmaze_user_agent: Option<String>,
}

impl ApiKeysFile {
    pub fn load() -> (Option<Self>, ApiKeysLoadStatus) {
        match Self::path() {
            Some(path) => Self::load_from(path),
            None => (
                None,
                ApiKeysLoadStatus {
                    path: None,
                    state: ApiKeysLoadState::Missing,
                },
            ),
        }
    }

    pub fn load_from(path: PathBuf) -> (Option<Self>, ApiKeysLoadStatus) {
        let (keys, state) = match fs::read_to_string(&path) {
            Ok(contents) => {
                #[cfg(unix)]
                if let Err(err) = restrict_to_owner(&path) {
                    eprintln!("Failed to restrict {} to its owner: {err}", path.display());
                }
                match toml::from_str(&contents) {
                    Ok(keys) => (Some(keys), ApiKeysLoadState::Loaded),
                    Err(_) => (None, ApiKeysLoadState::Unreadable),
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => (None, ApiKeysLoadState::Missing),
            Err(_) => (None, ApiKeysLoadState::Unreadable),
        };
        let status = ApiKeysLoadStatus {
            path: Some(path),
            state,
        };
        (keys, status)
    }

    pub fn save(&self) -> io::Result<PathBuf> {
        let path = Self::path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No config directory available")
        })?;
        self.save_to(&path)?;
        Ok(path)
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let payload = toml::to_string_pretty(self).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TOML encode error: {err}"),
            )
        })?;
        write_private_file(path, &payload)
    }

    pub fn path() -> Option<PathBuf> {
        paths::config_dir().map(|dir| dir.join("api_keys.toml"))
    }
}

/// Writes `contents` to a temporary file next to `path`, then renames it into
/// place, so a crash never leaves a half-written file. On Unix the file is
/// created readable by its owner only (0600). A symlinked `path` keeps its
/// link: the file it points to is the one replaced.
pub fn write_private_file(path: &Path, contents: &str) -> io::Result<()> {
    use std::io::Write as _;

    let path = match fs::canonicalize(path) {
        Ok(target) => target,
        Err(err) if err.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(err) => return Err(err),
    };
    let temp_path = path.with_extension("toml.tmp");
    // A leftover from an interrupted save may carry looser permissions.
    let _ = fs::remove_file(&temp_path);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&temp_path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp_path, &path)
}

/// Drops group and other permissions from a file written before KayaBot
/// created it with mode 0600.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    let private = mode & 0o600;
    if mode != private {
        fs::set_permissions(path, fs::Permissions::from_mode(private))?;
    }
    Ok(())
}

/// Older versions also read API keys from config.toml. Moves any key found
/// there into api_keys.toml, unless api_keys.toml already sets it, then drops
/// every credential from config.toml.
pub fn migrate_config_credentials(dir: &Path) -> io::Result<()> {
    let config_path = dir.join("config.toml");
    let contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    let mut config: toml::Table =
        toml::from_str(&contents).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let entries_before = config.len();
    let mut take = |key: &str| {
        config
            .remove(key)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let tmdb_bearer_token = take("tmdb_bearer_token");
    let tmdb_api_key = take("tmdb_api_key");
    let tvdb_api_key = take("tvdb_api_key");
    let omdb_api_key = take("omdb_api_key");
    let tvmaze_user_agent = take("tvmaze_user_agent");
    let tvmaze_api_key = take("tvmaze_api_key");
    // AniDB support is gone: drop its credentials rather than keep them.
    take("anidb_password");
    take("anidb_api_key");
    if config.len() == entries_before {
        return Ok(());
    }

    let keys_path = dir.join("api_keys.toml");
    let mut keys: ApiKeysFile = match fs::read_to_string(&keys_path) {
        Ok(contents) => toml::from_str(&contents)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
        Err(err) if err.kind() == io::ErrorKind::NotFound => ApiKeysFile::default(),
        Err(err) => return Err(err),
    };
    let mut moved = false;
    let mut fill = |slot: &mut Option<String>, value: Option<String>| {
        if slot.is_none() && value.is_some() {
            *slot = value;
            moved = true;
        }
    };
    // Both TMDB fields resolve to one credential: move them only as a pair.
    if keys.tmdb_bearer_token.is_none() && keys.tmdb_api_key.is_none() {
        fill(&mut keys.tmdb_bearer_token, tmdb_bearer_token);
        fill(&mut keys.tmdb_api_key, tmdb_api_key);
    }
    fill(&mut keys.tvdb_api_key, tvdb_api_key);
    fill(&mut keys.omdb_api_key, omdb_api_key);
    fill(
        &mut keys.tvmaze_user_agent,
        tvmaze_user_agent.or(tvmaze_api_key),
    );
    if moved {
        keys.save_to(&keys_path)?;
    }

    let remaining = toml::to_string_pretty(&config)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write_private_file(&config_path, &remaining)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn config_credentials_move_into_api_keys() {
        let temp = TempDir::new("migrate");
        fs::write(
            temp.join("config.toml"),
            "omdb_api_key = \"omdb-from-config\"\n\
             tvdb_api_key = \"tvdb-from-config\"\n\
             tvmaze_api_key = \"agent-from-config\"\n\
             anidb_password = \"anidb-password\"\n\
             original_files = []\n\
             [format_options]\n\
             include_episode_title = true\n",
        )
        .expect("write config.toml");
        fs::write(temp.join("api_keys.toml"), "tvdb_api_key = \"tvdb-kept\"\n")
            .expect("write api_keys.toml");

        migrate_config_credentials(&temp).expect("migration");

        let (keys, _) = ApiKeysFile::load_from(temp.join("api_keys.toml"));
        let keys = keys.expect("api_keys.toml parses");
        assert_eq!(keys.omdb_api_key.as_deref(), Some("omdb-from-config"));
        assert_eq!(keys.tvdb_api_key.as_deref(), Some("tvdb-kept"));
        assert_eq!(keys.tvmaze_user_agent.as_deref(), Some("agent-from-config"));
        let config = fs::read_to_string(temp.join("config.toml")).expect("read config.toml");
        for gone in ["from-config", "anidb", "api_key"] {
            assert!(
                !config.contains(gone),
                "{gone} left in config.toml:\n{config}"
            );
        }
        assert!(config.contains("original_files"), "{config}");
        assert!(config.contains("include_episode_title"), "{config}");

        // A second start finds nothing left to move.
        migrate_config_credentials(&temp).expect("second migration");
        assert_eq!(
            fs::read_to_string(temp.join("config.toml")).expect("read config.toml"),
            config
        );
    }

    #[test]
    fn migration_without_config_toml_writes_nothing() {
        let temp = TempDir::new("migrate-none");

        migrate_config_credentials(&temp).expect("migration");

        assert!(!temp.join("api_keys.toml").exists());
        assert!(!temp.join("config.toml").exists());
    }

    #[test]
    fn a_missing_or_broken_file_reports_its_state() {
        let temp = TempDir::new("api-keys-state");
        let path = temp.join("api_keys.toml");

        let (keys, status) = ApiKeysFile::load_from(path.clone());
        assert!(keys.is_none());
        assert_eq!(status.state, ApiKeysLoadState::Missing);

        fs::write(&path, "tvdb_api_key = [").expect("write broken file");
        let (keys, status) = ApiKeysFile::load_from(path.clone());
        assert!(keys.is_none());
        assert_eq!(status.state, ApiKeysLoadState::Unreadable);
        assert_eq!(status.path.as_deref(), Some(path.as_path()));
    }

    #[cfg(unix)]
    #[test]
    fn api_keys_file_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new("api-keys-mode");
        let path = temp.join("api_keys.toml");
        let mode = |path: &Path| fs::metadata(path).expect("metadata").permissions().mode() & 0o777;

        ApiKeysFile::default().save_to(&path).expect("save");
        assert_eq!(mode(&path), 0o600);

        // A file written by an older version gets tightened on load.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("chmod");
        let (keys, _) = ApiKeysFile::load_from(path.clone());
        assert!(keys.is_some());
        assert_eq!(mode(&path), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn saving_through_a_symlink_keeps_the_link() {
        let temp = TempDir::new("api-keys-symlink");
        let target = temp.join("dotfiles-api_keys.toml");
        let link = temp.join("api_keys.toml");
        fs::write(&target, "").expect("write target");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let keys = ApiKeysFile {
            omdb_api_key: Some("omdb-key".to_string()),
            ..ApiKeysFile::default()
        };
        keys.save_to(&link).expect("save");

        assert!(
            fs::symlink_metadata(&link)
                .expect("link metadata")
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::read_to_string(&target)
                .expect("read target")
                .contains("omdb-key")
        );
    }
}
