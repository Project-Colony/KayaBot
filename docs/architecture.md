# Architecture

KayaBot is one Rust crate with a library (`kayabot`, `src/lib.rs`) and a
desktop program (`src/main.rs`). The library holds everything that does not
draw a window: parsing, matching, the metadata clients, name templates and the
rename engine. The program is the window, its state and the settings files.

## Source layout

| Path | What it does |
|---|---|
| `src/main.rs` | the window (eframe/egui): the sidebar, the Rename view, the Settings page, the settings and key files, and the export to CSV/JSON |
| `src/matching/parsing.rs` | `parse_filename`: splits a file name into tokens and reads a title, a season and episode, and a year |
| `src/matching/mod.rs` | `match_files`: turns each parsed name into a `MatchResult` with its candidates (episode, movie or series), a status and a confidence |
| `src/matching/advanced.rs` | `normalize_name` and `rank_candidates`, a Jaro-Winkler and Levenshtein ranking. Exported and tested, not called by the program |
| `src/metadata/provider.rs` | the `MetadataProvider` trait (`search_title`, `fetch_episode_list`, `fetch_movie_details`) and the `MetadataSource` enum |
| `src/metadata/providers/` | one client per source: `tmdb.rs`, `thetvdb.rs`, `tvmaze.rs`, `omdb.rs` |
| `src/metadata/aggregate.rs` | `MetadataPipeline`: queries the active sources in parallel, merges and ranks their answers |
| `src/metadata/models.rs` | `TitleMatch`, `EpisodeMatch`, `MovieMatch` and the `ExternalIds` and extras they carry |
| `src/metadata/locale.rs` | `MetadataLocale`, the language and region sent to the sources |
| `src/metadata/error.rs` | `MetadataError`, the errors a client reports to the window |
| `src/formatting/mod.rs` | name templates, title clean-up, and `format_options.toml` |
| `src/rename.rs` | the rename engine |
| `src/paths.rs` | the configuration folder |

`src/main.rs` declares `formatting`, `matching`, `metadata` and `paths` as its
own modules rather than importing them from the library, so those four are
compiled twice; only `rename` comes from `kayabot::`. `cargo run` starts the
program.

## From an old name to a new one

1. **Add files.** The Rename view takes files or a folder from the system file
   dialog (`rfd`).
2. **Parse.** `matching::match_files` reads each name. `S01E02`, `S01 E02`,
   `1x02`, a three-digit `102` and `Season 1 Episode 2` (or `Saison`) give a
   season and episode; a four-digit number from 1900 to 2099 gives a year.
   Quality, codec, audio and release-group tags are dropped from the title. A
   season and episode make the file an episode; a year without them makes it a
   movie. You can switch the type in the view.
3. **Pick the sources.** Series default to TheTVDB with TVmaze as its partner,
   movies to TMDB with OMDb. The active list is the chosen source plus its
   partners (TMDB chosen for a series brings both TheTVDB and TVmaze). The
   "force source" checkbox keeps only the chosen one.
4. **Search.** `MetadataPipeline::search_title` sends the title (plus the year
   for a movie) to every active source at once, on scoped threads, and waits
   for all of them. A source that fails or has no key is left out; the search
   fails only when every source does.
5. **Merge and rank.** Results that share an external ID (IMDb, TMDB, TheTVDB,
   TVmaze, OMDb) or whose titles are at least 0.9 similar with compatible years
   become one entry. Each entry scores
   `0.55 * trust + 0.35 * title similarity + 0.10 * source rating`, where trust
   is 0.9 for TMDB and TheTVDB, 0.75 for TVmaze and 0.7 for OMDb.
6. **Episode.** For an episode, the best title's episode list is fetched from
   the source that returned it, and the entry with the parsed season and number
   is kept.
7. **Format.** `formatting` renders the template for each file (see below).
8. **Preview, dry run, rename.** `rename::apply` builds the target next to the
   original and keeps the extension. With the dry run on, it resolves the name
   and touches nothing; otherwise it renames, then the list reloads from the new
   names. Each file's outcome feeds the summary, which can be exported.

The provider clients use blocking HTTP with a 10 second timeout, called from
the window's update loop, so the window does not redraw until a search ends.

## Name templates

| Token | Value |
|---|---|
| `{n}` | the series or movie title (also `{name}`, `{title}`) |
| `{t}` | the episode title (also `{episodetitle}`) |
| `{s}` | the season number (also `{season}`) |
| `{e}` | the episode number (also `{episode}`) |
| `{y}` | the year (also `{year}`) |

Digits after a token pad the number: `{s00}` gives `01`. The defaults are
`{n} {s}x{e} - {t}` for episodes and `{n} ({y})` for movies; turning off the
episode title or the year in the view switches them to `{n} {s}x{e}` and `{n}`.
When a token has no value, it is dropped together with the separators around
it, so a movie without a year becomes `Title`, not `Title ()`. Titles are
cleaned before use: every character that is not a letter or a digit becomes a
space (so `Grey's` becomes `Grey s`, and no character a file name cannot hold
survives), technical tags (`1080p`, `BluRay` and so on) are removed, and runs
of spaces collapse to one.

## Rename engine

`src/rename.rs` builds the target path next to the original, picks the first
free name (` (1)`, ` (2)` and so on, up to 9999) and renames the file. It never
overwrites an existing file, a hard link to the original included. Two files of
one batch never get names that differ only in letter case. A case-only rename
goes through a temporary name; a move across devices copies into a new file,
syncs it, then removes the original.

## Configuration files

`paths::app_config_dir` is `Colony/KayaBot` under `dirs::config_dir()` on Linux
and macOS and under `dirs::data_local_dir()` (`%LOCALAPPDATA%`) on Windows.

- `api_keys.toml`: read at start and by the reload button of the API
  configuration section of Settings; written by its save button through
  `write_private_file`, which writes a temporary file with mode 0600 on Unix
  and renames it into place. Environment variables override each key.
- `preferences.toml`: written whenever a setting changes and when the window
  closes. It also reads an older flat format. Only the theme and the metadata language and region are
  applied; the other settings are stored and have no effect yet.
- `format_options.toml`: the last template choice.
- `config.toml`: only from older versions. Before the window opens,
  `migrate_config_credentials` moves its keys into `api_keys.toml` and removes
  them, AniDB credentials included.

## Tests

- Unit tests sit next to the code: templates, the rename engine, the pipeline,
  each provider client against the recorded responses in `tests/fixtures/`,
  and in `main.rs` the key migration, the private file writes and the export.
- `tests/matching_snapshots.rs` runs the parser over an episode, a movie and
  a bare title and compares the result with an `insta` snapshot. The empty
  files in `tests/fixtures/media_samples` feed the rename and Rename view
  tests.
- `tests/integration_api.rs` calls the real services. Its tests are ignored by
  default and need the keys from `.env.example`:

  ```bash
  KAYABOT_TMDB_API_KEY=... KAYABOT_OMDB_API_KEY=... KAYABOT_TVDB_API_KEY=... \
    cargo test --test integration_api -- --ignored
  ```

  `KAYABOT_TMDB_BEARER_TOKEN` works in place of `KAYABOT_TMDB_API_KEY`.
