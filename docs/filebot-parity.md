# FileBot parity

FileBot is the reference for what a media renamer should do. This page lists
the FileBot features KayaBot aims to cover and where each one stands today, so
nobody has to read the code to find out.

**Has it** means it works in the current program. **Partly** means a smaller
version exists. **Not yet** means there is no code for it.

## Renaming and organising

| Feature | KayaBot today |
|---|---|
| Rename from a template | **Has it.** Presets and custom templates for episodes and movies |
| Keep the extension | **Has it** |
| Handle name collisions | **Has it.** Never overwrites; a taken name gets ` (1)`, ` (2)` and so on, and two files of a batch never collide |
| Preview and dry run | **Has it.** Every new name is shown first, and a dry run renames nothing |
| Move files into series, season or movie folders | **Not yet.** Files are renamed in their own folder |
| Rename subtitles with their video | **Not yet** |
| Write `.nfo` files or file tags | **Not yet** |
| Check SFV or other checksums | **Not yet** |

## Matching

| Feature | KayaBot today |
|---|---|
| Read season, episode and year from a file name | **Has it.** `S01E02`, `S01 E02`, `1x02`, `102`, `Season 1 Episode 2`, a year from 1900 to 2099 |
| Tell an episode from a movie | **Has it**, from the parsed name; you can switch it |
| Match series and episodes | **Has it.** Title search, then the episode by season and number |
| Match movies | **Has it.** Title and year |
| Choose between ambiguous results | **Partly.** Candidates are listed with their score and you can pick another one, or correct a file's title, season and episode by hand. There is no interactive prompt during a batch |
| Match by file hash | **Not yet** |
| Editions and cuts, multi-episode files, specials | **Not yet** |
| Filter files by rules | **Not yet** |

## Metadata

| Feature | KayaBot today |
|---|---|
| Several sources with a priority per content type | **Has it.** TheTVDB and TVmaze for series, TMDB and OMDb for movies; see [metadata-sources.md](metadata-sources.md) |
| Merge results across sources | **Has it.** By shared IDs and title similarity |
| Fall back when a source fails | **Has it.** Partners are queried at the same time and a failing source is left out |
| Official title, year and episode title | **Has it** |
| Synopsis and genres | **Partly.** Shown next to a match, not used in names |
| Language and region of titles | **Has it** for TMDB and TheTVDB |
| Cast, artwork, collections | **Not yet** |

## Templates

KayaBot reads FileBot-style tokens, a small subset of FileBot's format
language:

| Token | KayaBot today |
|---|---|
| `{n}` title, `{t}` episode title, `{y}` year | **Has it** |
| `{s}`, `{e}` and padded forms such as `{s00}`, `{e00}` | **Has it** |
| Drop a missing value with its separator | **Has it** |
| `{airdate}` and other FileBot bindings | **Not yet** |
| Expressions and conditions inside a template | **Not yet** |
