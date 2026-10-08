# KayaBot

KayaBot is a desktop media renamer written in Rust (eframe/egui). It parses
movie and TV episode file names, matches them against online metadata sources
and renames the files to a consistent format, in the spirit of FileBot.

> **Status:** early prototype. The Rename view works end to end: file name
> parsing, matching against TMDB, TheTVDB, TVmaze, OMDb and AniDB with
> fallback, a preview, a dry run, the rename itself and CSV/JSON export. The
> Episodes, Subtitles, SFV and Filter screens are placeholders, and some
> preferences are saved but not used yet. There is no CI and no release yet.

## What it does

- Parses file names (`S01E02`, `1x02`, year, quality tags and so on).
- Queries several metadata providers in parallel and keeps the best match.
- Previews new names from default or custom templates for series and movies.
- Renames files with dry run and collision handling.
- Caches metadata responses locally.
- Ships a small CLI, `preload_metadata_cache`, to seed the cache from a JSON
  file (`--help` for the format).

## Build from source

You need a recent stable Rust toolchain (Rust 1.88 or newer).

```bash
cargo run --release
```

## Configuration

KayaBot stores its files in:

- Linux: `~/.config/Colony/KayaBot/`
- macOS: `~/Library/Application Support/Colony/KayaBot/`
- Windows: `%LOCALAPPDATA%\Colony\KayaBot\`

Put API keys in `api_keys.toml` in that folder, or set them as environment
variables. Environment variables take precedence over the files.

```toml
tmdb_bearer_token = "..."
tvdb_api_key = "..."
omdb_api_key = "..."
anidb_api_key = "..."
tvmaze_user_agent = "KayaBot"
```

Supported environment variables are listed in [`.env.example`](.env.example).

## Privacy

To find metadata, KayaBot sends the titles, years and episode numbers it parses
from your file names to the providers you configure: TMDB, TheTVDB, TVmaze,
OMDb and AniDB. API keys are stored in plain text in `api_keys.toml`. Metadata
responses are cached in `metadata_cache.json` in the configuration folder.

## Documentation

See [`docs/`](docs/) for technical notes (currently in French).

## License

KayaBot is licensed under the GNU General Public License v3.0 or later. See
[LICENSE](LICENSE).
