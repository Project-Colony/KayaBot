# Architecture

KayaBot is one Rust crate with a library (`kayabot`, `src/lib.rs`) and a
desktop program (`src/main.rs`). The library holds everything, the interface
included; `main.rs` only answers `--version`, picks the configuration folder
and opens the window.

## Source layout

| Path | What it does |
|---|---|
| `src/main.rs` | the entry point: `--version`, the configuration folder, then `ui::run` |
| `src/ui/mod.rs` | `run`: applies the preferences, registers the fonts and starts iced |
| `src/ui/app.rs` | `App`, every `Message` it handles, the frame around the views, file drops and keyboard shortcuts |
| `src/ui/sidebar.rs` | the sidebar: the KayaBot button that opens Preferences, then the Rename entry |
| `src/ui/preferences.rs` | the Preferences page: General, Appearance and Accessibility from colony-ui, then Connections, Renaming and About |
| `src/ui/rename/mod.rs` | the Rename view's state and messages, the rename itself, and the name preview |
| `src/ui/rename/files.rs`, `preview.rs`, `details.rs`, `summary.rs` | the file list, the proposed names with the lookup and naming options, the match dialog, and the summary with its export and the confirmation dialog |
| `src/ui/widgets.rs` | KayaBot's own buttons, cards, fields and dialogs, styled from the palette, and its Nerd Font glyphs |
| `src/i18n/` | `t` and `tf`, with `en.json` and `fr.json`; the Preferences skeleton and theme labels come from `colony_ui::i18n` |
| `src/config.rs` | `Config`, saved as `preferences.json`, and the conversion of the older `preferences.toml` |
| `src/api_keys.rs` | the keys from the environment and `api_keys.toml`, `write_private_file`, and `migrate_config_credentials` |
| `src/lookup.rs` | which sources a lookup asks, what it asks them, and the lookups the interface runs off its thread |
| `src/export.rs` | the rename summary as CSV or JSON |
| `src/paths.rs` | the configuration folder, from `colony_ui::paths`, and the one-time copy from older locations |
| `src/matching/parsing.rs` | `parse_filename`: splits a file name into tokens and reads a title, a season and episode, and a year |
| `src/matching/mod.rs` | `match_files`: turns each parsed name into a `MatchResult` with its candidates (episode, movie or series), a status and a confidence |
| `src/matching/advanced.rs` | `normalize_name` and `rank_candidates`, a Jaro-Winkler and Levenshtein score. The pipeline uses it for the title similarity of each result |
| `src/metadata/provider.rs` | the `MetadataProvider` trait (`search_title`, `fetch_episode_list`, `fetch_movie_details`) and the `MetadataSource` enum |
| `src/metadata/providers/` | one client per source: `tmdb.rs`, `thetvdb.rs`, `tvmaze.rs`, `omdb.rs` |
| `src/metadata/aggregate.rs` | `MetadataPipeline`: queries the active sources in parallel, merges and ranks their answers |
| `src/metadata/models.rs` | `TitleMatch`, `EpisodeMatch`, `MovieMatch` and the `ExternalIds` and extras they carry |
| `src/metadata/locale.rs` | `MetadataLocale`, the language and region sent to the sources |
| `src/metadata/error.rs` | `MetadataError`, the errors a client reports to the window |
| `src/formatting.rs` | name templates, title clean-up, and `format_options.toml` |
| `src/rename.rs` | the rename engine |

## The interface

The window is built with [iced](https://iced.rs) 0.14 and
[colony-ui](https://crates.io/crates/colony-ui), the design system every
Colony program shares. Colours come from the active Colony palette, text from
the three Colony fonts through `Typography::current()` (so the dyslexia font
and both text sizes reach every widget), and sizes go through `sz()`. The
built-in iced widgets get the palette through `App::theme`.

`App` follows the iced pattern: `update` takes a `Message` and changes the
state, `view` draws it. The Rename view is a component of its own
(`ui::rename::Rename`) with its own `Message`, wrapped in `Message::Rename`.
Preferences replaces the content area, never a window or a dialog, and the
KayaBot button and the page's Close button send the same
`Message::TogglePreferences`. A change in Preferences applies at once and is
saved; there is no Save button.

## From an old name to a new one

1. **Add files.** The Rename view takes files or a folder from the system file
   dialog (`rfd`), or files and folders dropped on the window.
2. **Parse.** `matching::match_files` reads each name. `S01E02`, `S01 E02`,
   `1x02`, a three-digit `102` and `Season 1 Episode 2` (or `Saison`) give a
   season and episode; a four-digit number from 1900 to 2099 gives a year.
   Quality, codec, audio and release-group tags are dropped from the title. A
   season and episode make the file an episode; a year without them makes it a
   movie. You can switch the type in the view.
3. **Pick the sources.** Series default to TheTVDB with TVmaze as its partner,
   movies to TMDB with OMDb. The active list is the chosen source plus its
   partners (TMDB chosen for a series brings both TheTVDB and TVmaze). The
   "Use this source only" checkbox keeps only the chosen one.
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
   and touches nothing; otherwise, once the user confirms (unless that is
   turned off in Preferences), it renames, then the list reloads from the new
   names. Each file's outcome feeds the summary, which can be exported.

The provider clients use blocking HTTP with a 10 second timeout. The interface
never calls them itself: `Rename::run` hands the lookup to a worker thread and
gets the answer back as a message, and Match and Fetch stay disabled while one
runs. A match started before the file list changed is dropped when it lands.

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

`paths::config_dir` is `colony_ui::paths::config_dir("KayaBot")`:
`~/.config/Colony/KayaBot` on Linux, `%LOCALAPPDATA%\Colony\KayaBot` on
Windows and `~/Library/Application Support/Colony/KayaBot` on macOS.

Before anything is read, `paths::init` checks that folder. When it is empty and
an older location has files (first the Windows roaming profile,
`%APPDATA%\Colony\KayaBot`, then `~/.kayabot`), it copies them over and writes
a `.migrated` marker. The older folder is never changed. A copy that fails is
undone, and the older folder is used for that run.

- `preferences.json`: `Config`, which is colony-ui's `StandardPreferences`
  (theme, accent, text sizes, high contrast, dyslexia font, reduced motion)
  plus the interface language, the metadata language and region, the two
  confirmations and the dry-run default. Each field is read on its own, so a
  bad value only resets that field. Written whenever a preference changes.
  When it is missing, the `preferences.toml` of an older version, flat or in
  sections, is read once and saved in the new format; its settings that never
  did anything are dropped.
- `api_keys.toml`: read at start; written as the keys are typed in
  Preferences, Connections, through `write_private_file`, which writes a
  temporary file with mode 0600 on Unix and renames it into place.
  Environment variables override each key.
- `format_options.toml`: the name templates and the year and episode title
  choices.
- `config.toml`: only from older versions. Before the window opens,
  `migrate_config_credentials` moves its keys into `api_keys.toml` and removes
  them, AniDB credentials included.

## Tests

- Unit tests sit next to the code: templates, the rename engine, the pipeline,
  each provider client against the recorded responses in `tests/fixtures/`,
  the key migration and the private file writes, the export, the preferences
  and their conversion, the copy from older folders, and the interface's
  `update` with a mock source (`ui/app.rs`, `ui/rename/tests.rs`). The unit
  tests write to a temporary folder, never to the real profile.
- `i18n` checks that `en.json` and `fr.json` have the same keys and
  placeholders and that every key the interface asks for exists, and
  `ui/widgets.rs` that every glyph exists in the Nerd Font.
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
