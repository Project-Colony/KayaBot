use std::env;

use kayabot::metadata::providers::{omdb::OmdbClient, thetvdb::TheTvDbClient, tmdb::TmdbClient};

#[test]
#[ignore = "requires TMDB_API_KEY"]
fn tmdb_headers_include_api_key() {
    let key = env::var("TMDB_API_KEY").expect("TMDB_API_KEY must be set");
    let client = TmdbClient::new(key.clone(), None);
    let headers = client.auth_headers();
    let auth_header = headers
        .get("Authorization")
        .expect("Authorization header missing");
    assert!(auth_header.contains(&key));
}

#[test]
#[ignore = "requires OMDB_API_KEY"]
fn omdb_headers_include_api_key() {
    let key = env::var("OMDB_API_KEY").expect("OMDB_API_KEY must be set");
    let client = OmdbClient::new(key.clone());
    let headers = client.auth_headers();
    let header_value = headers
        .get("X-OMDb-API-Key")
        .expect("X-OMDb-API-Key header missing");
    assert_eq!(header_value, &key);
}

#[test]
#[ignore = "requires THETVDB_API_KEY"]
fn thetvdb_headers_include_api_key() {
    let key = env::var("THETVDB_API_KEY").expect("THETVDB_API_KEY must be set");
    let client = TheTvDbClient::new(key.clone(), None);
    let headers = client.auth_headers();
    let auth_header = headers
        .get("Authorization")
        .expect("Authorization header missing");
    assert!(auth_header.contains(&key));
}
