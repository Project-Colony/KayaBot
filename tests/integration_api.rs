use std::env;

use kayabot::metadata::provider::MetadataProvider;
use kayabot::metadata::providers::{omdb::OmdbClient, thetvdb::TheTvDbClient, tmdb::TmdbClient};

/// The first of `names` that is set to a non-blank value, the way KayaBot
/// itself reads the keys listed in `.env.example`.
fn key_from_env(names: &[&str]) -> String {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| panic!("set {}", names.join(" or ")))
}

/// Accepts either a v3 API key or a v4 read access token.
#[test]
#[ignore = "requires KAYABOT_TMDB_BEARER_TOKEN or KAYABOT_TMDB_API_KEY"]
fn tmdb_search_accepts_the_configured_key() {
    let key = key_from_env(&["KAYABOT_TMDB_BEARER_TOKEN", "KAYABOT_TMDB_API_KEY"]);
    let mut client = TmdbClient::new(key, None);
    let results = client
        .search_title("Breaking Bad")
        .expect("TMDB search failed");
    assert!(!results.is_empty());
}

#[test]
#[ignore = "requires KAYABOT_OMDB_API_KEY"]
fn omdb_search_accepts_the_configured_key() {
    let key = key_from_env(&["KAYABOT_OMDB_API_KEY"]);
    let mut client = OmdbClient::new(key);
    let results = client
        .search_title("The Matrix")
        .expect("OMDb search failed");
    assert!(!results.is_empty());
}

/// TheTVDB trades the key for a session token on the first request.
#[test]
#[ignore = "requires KAYABOT_TVDB_API_KEY"]
fn thetvdb_search_accepts_the_configured_key() {
    let key = key_from_env(&["KAYABOT_TVDB_API_KEY"]);
    let mut client = TheTvDbClient::new(key, None);
    let results = client
        .search_title("Breaking Bad")
        .expect("TheTVDB search failed");
    assert!(!results.is_empty());
}
