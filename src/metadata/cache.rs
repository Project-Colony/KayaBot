#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::metadata::models::{EpisodeMatch, TitleMatch};
use crate::metadata::provider::MetadataSource;
use crate::paths;

const DEFAULT_TTL_SECONDS: u64 = 60 * 60 * 24;
const CACHE_ENV_PATH: &str = "KAYABOT_CACHE_PATH";
const CACHE_ENV_TTL: &str = "KAYABOT_CACHE_TTL_SECONDS";
const CACHE_ENV_REFRESH: &str = "KAYABOT_CACHE_REFRESH_ON_HIT";

#[derive(Debug, Clone, Copy)]
pub struct CachePolicy {
    pub ttl_seconds: u64,
    pub refresh_on_hit: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            ttl_seconds: DEFAULT_TTL_SECONDS,
            refresh_on_hit: true,
        }
    }
}

impl CachePolicy {
    pub fn from_env() -> Self {
        let mut policy = Self::default();
        if let Ok(value) = std::env::var(CACHE_ENV_TTL)
            && let Ok(parsed) = value.parse::<u64>()
        {
            policy.ttl_seconds = parsed;
        }
        if let Ok(value) = std::env::var(CACHE_ENV_REFRESH)
            && let Ok(parsed) = value.parse::<bool>()
        {
            policy.refresh_on_hit = parsed;
        }
        policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct CacheKey {
    source: MetadataSource,
    key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    value: T,
    expires_at: u64,
    refreshed_at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CacheStorage {
    title_searches: HashMap<CacheKey, CacheEntry<Vec<TitleMatch>>>,
    episode_lists: HashMap<CacheKey, CacheEntry<Vec<EpisodeMatch>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataCacheSeed {
    pub title_searches: Vec<MetadataCacheSeedEntry<Vec<TitleMatch>>>,
    pub episode_lists: Vec<MetadataCacheSeedEntry<Vec<EpisodeMatch>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCacheSeedEntry<T> {
    pub source: MetadataSource,
    pub key: String,
    pub value: T,
}

#[derive(Debug)]
pub struct MetadataCache {
    title_searches: HashMap<CacheKey, CacheEntry<Vec<TitleMatch>>>,
    episode_lists: HashMap<CacheKey, CacheEntry<Vec<EpisodeMatch>>>,
    policy: CachePolicy,
    path: PathBuf,
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataCache {
    pub fn new() -> Self {
        Self::with_policy(CachePolicy::from_env())
    }

    pub fn with_policy(policy: CachePolicy) -> Self {
        let path = Self::default_cache_path();
        let mut cache = Self {
            title_searches: HashMap::new(),
            episode_lists: HashMap::new(),
            policy,
            path,
        };
        cache.load_from_disk();
        cache.clear_expired();
        cache.persist();
        cache
    }

    pub fn get_title_search(
        &mut self,
        source: MetadataSource,
        query: &str,
    ) -> Option<Vec<TitleMatch>> {
        let key = CacheKey {
            source,
            key: query.to_string(),
        };
        let now = current_timestamp();
        let (value, changed) = Self::read_entry(
            &mut self.title_searches,
            &key,
            now,
            self.policy.ttl_seconds,
            self.policy.refresh_on_hit,
        );
        if changed {
            self.persist();
        }
        value
    }

    pub fn put_title_search(
        &mut self,
        source: MetadataSource,
        query: &str,
        results: Vec<TitleMatch>,
    ) {
        let key = CacheKey {
            source,
            key: query.to_string(),
        };
        let now = current_timestamp();
        Self::write_entry(
            &mut self.title_searches,
            key,
            results,
            now,
            self.policy.ttl_seconds,
        );
        self.persist();
    }

    pub fn get_episode_list(
        &mut self,
        source: MetadataSource,
        title_id: &str,
    ) -> Option<Vec<EpisodeMatch>> {
        let key = CacheKey {
            source,
            key: title_id.to_string(),
        };
        let now = current_timestamp();
        let (value, changed) = Self::read_entry(
            &mut self.episode_lists,
            &key,
            now,
            self.policy.ttl_seconds,
            self.policy.refresh_on_hit,
        );
        if changed {
            self.persist();
        }
        value
    }

    pub fn put_episode_list(
        &mut self,
        source: MetadataSource,
        title_id: &str,
        episodes: Vec<EpisodeMatch>,
    ) {
        let key = CacheKey {
            source,
            key: title_id.to_string(),
        };
        let now = current_timestamp();
        Self::write_entry(
            &mut self.episode_lists,
            key,
            episodes,
            now,
            self.policy.ttl_seconds,
        );
        self.persist();
    }

    pub fn clear(&mut self) {
        self.title_searches.clear();
        self.episode_lists.clear();
        self.persist();
    }

    pub fn merge_seed(&mut self, seed: MetadataCacheSeed) -> usize {
        let mut inserted = 0;
        let now = current_timestamp();
        for entry in seed.title_searches {
            let key = CacheKey {
                source: entry.source,
                key: entry.key,
            };
            Self::write_entry(
                &mut self.title_searches,
                key,
                entry.value,
                now,
                self.policy.ttl_seconds,
            );
            inserted += 1;
        }
        for entry in seed.episode_lists {
            let key = CacheKey {
                source: entry.source,
                key: entry.key,
            };
            Self::write_entry(
                &mut self.episode_lists,
                key,
                entry.value,
                now,
                self.policy.ttl_seconds,
            );
            inserted += 1;
        }
        if inserted > 0 {
            self.persist();
        }
        inserted
    }

    fn read_entry<T: Clone>(
        map: &mut HashMap<CacheKey, CacheEntry<T>>,
        key: &CacheKey,
        now: u64,
        ttl_seconds: u64,
        refresh_on_hit: bool,
    ) -> (Option<T>, bool) {
        let Some(entry) = map.get_mut(key) else {
            return (None, false);
        };
        if entry.expires_at <= now {
            map.remove(key);
            return (None, true);
        }
        let mut changed = false;
        if refresh_on_hit {
            entry.refreshed_at = now;
            entry.expires_at = now + ttl_seconds;
            changed = true;
        }
        (Some(entry.value.clone()), changed)
    }

    fn write_entry<T: Clone>(
        map: &mut HashMap<CacheKey, CacheEntry<T>>,
        key: CacheKey,
        value: T,
        now: u64,
        ttl_seconds: u64,
    ) {
        let entry = CacheEntry {
            value,
            expires_at: now + ttl_seconds,
            refreshed_at: now,
        };
        map.insert(key, entry);
    }

    fn clear_expired(&mut self) {
        let now = current_timestamp();
        self.title_searches
            .retain(|_, entry| entry.expires_at > now);
        self.episode_lists.retain(|_, entry| entry.expires_at > now);
    }

    fn load_from_disk(&mut self) {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(storage) = serde_json::from_str::<CacheStorage>(&contents) else {
            eprintln!("Failed to parse metadata cache at {}", self.path.display());
            return;
        };
        self.title_searches = storage.title_searches;
        self.episode_lists = storage.episode_lists;
    }

    fn persist(&self) {
        let Some(parent) = self.path.parent() else {
            return;
        };
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!(
                "Failed to create cache directory {}: {err}",
                parent.display()
            );
            return;
        }
        let storage = CacheStorage {
            title_searches: self.title_searches.clone(),
            episode_lists: self.episode_lists.clone(),
        };
        let Ok(payload) = serde_json::to_string_pretty(&storage) else {
            eprintln!("Failed to serialize metadata cache.");
            return;
        };
        let temp_path = self.path.with_extension("json.tmp");
        if let Err(err) = fs::write(&temp_path, payload) {
            eprintln!("Failed to write metadata cache: {err}");
            return;
        }
        if let Err(err) = fs::rename(&temp_path, &self.path) {
            eprintln!("Failed to finalize metadata cache: {err}");
        }
    }

    fn default_cache_path() -> PathBuf {
        if let Ok(path) = std::env::var(CACHE_ENV_PATH) {
            return PathBuf::from(path);
        }
        if let Some(config_dir) = paths::app_config_dir() {
            return config_dir.join("metadata_cache.json");
        }
        PathBuf::from("metadata_cache.json")
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
