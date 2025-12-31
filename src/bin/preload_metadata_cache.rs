mod metadata;
mod paths {
    pub use kayabot::paths::*;
}

use std::fs;
use std::path::PathBuf;

use metadata::cache::{CachePolicy, MetadataCache, MetadataCacheSeed};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut input_path: Option<PathBuf> = None;
    let mut ttl_override: Option<u64> = None;
    let mut refresh_override: Option<bool> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" | "-i" => {
                let Some(value) = args.next() else {
                    eprintln!("Missing value for --input");
                    std::process::exit(1);
                };
                input_path = Some(PathBuf::from(value));
            }
            "--ttl-seconds" => {
                let Some(value) = args.next() else {
                    eprintln!("Missing value for --ttl-seconds");
                    std::process::exit(1);
                };
                let parsed = value.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!("Invalid ttl seconds: {value}");
                    std::process::exit(1);
                });
                ttl_override = Some(parsed);
            }
            "--no-refresh" => {
                refresh_override = Some(false);
            }
            "--refresh" => {
                refresh_override = Some(true);
            }
            "--cache-path" => {
                let Some(value) = args.next() else {
                    eprintln!("Missing value for --cache-path");
                    std::process::exit(1);
                };
                unsafe {
                    std::env::set_var("KAYABOT_CACHE_PATH", value);
                }
            }
            "--help" | "-h" => {
                print_usage();
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                std::process::exit(1);
            }
        }
    }

    let Some(input_path) = input_path else {
        eprintln!("Missing --input file path.");
        print_usage();
        std::process::exit(1);
    };

    let payload = fs::read_to_string(&input_path).unwrap_or_else(|err| {
        eprintln!("Failed to read {}: {err}", input_path.display());
        std::process::exit(1);
    });

    let seed: MetadataCacheSeed = serde_json::from_str(&payload).unwrap_or_else(|err| {
        eprintln!("Failed to parse seed file: {err}");
        std::process::exit(1);
    });

    let mut policy = CachePolicy::from_env();
    if let Some(ttl) = ttl_override {
        policy.ttl_seconds = ttl;
    }
    if let Some(refresh_on_hit) = refresh_override {
        policy.refresh_on_hit = refresh_on_hit;
    }

    let mut cache = MetadataCache::with_policy(policy);
    let inserted = cache.merge_seed(seed);
    println!("Seeded {inserted} cache entries.");
}

fn print_usage() {
    eprintln!(
        "Usage: preload_metadata_cache --input <seed.json> [--ttl-seconds N] [--no-refresh] [--cache-path path]"
    );
    eprintln!("Seed file format:");
    eprintln!("{{");
    eprintln!(
        "  \"title_searches\": [{{\"source\": \"TheMovieDb\", \"key\": \"query\", \"value\": [TitleMatch...]}}],"
    );
    eprintln!(
        "  \"episode_lists\": [{{\"source\": \"TheMovieDb\", \"key\": \"id\", \"value\": [EpisodeMatch...]}}]"
    );
    eprintln!("}}");
}
