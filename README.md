<div align="center">

**A desktop media renamer for movie and TV libraries: it looks your files up in TMDB, TheTVDB, TVmaze and OMDb and gives them clean, consistent names.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-multimedia-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)

Downloaded episodes and movies arrive with names like
`show.name.s01e02.1080p.web.x264-grp.mkv`. A bulk renamer can clean up the
punctuation but does not know the episode's title, and FileBot, the usual tool
for the job, needs a paid licence. KayaBot parses the file names, asks several
online databases at once, and shows every new name before it touches a file.

> **Status:** early prototype, and no release has been published yet. The
> Rename view works end to end: parsing, matching against TMDB, TheTVDB,
> TVmaze and OMDb, a preview, a dry run, the rename itself and a CSV or JSON
> summary. The parser, the name templates, the rename engine and the four
> provider clients have automated tests, the clients against recorded
> responses; the window itself is tested by hand only. The Episodes,
> Subtitles, SFV and Filter screens in the sidebar are empty placeholders, the
> interface mixes English and French text, and most settings are saved but do
> nothing yet: only the theme and the metadata language and region take effect.

## Why KayaBot

- **A bulk renamer or a regex** tidies the punctuation, but you still type every
  episode title. KayaBot fetches the titles and the year.
- **FileBot** needs a paid licence. KayaBot is free software under
  GPL-3.0-or-later.
- **One database is never complete.** KayaBot queries more than one source in
  parallel and merges their answers by shared IDs (IMDb, TMDB, TheTVDB, TVmaze)
  and title similarity, so a show missing from one source still matches.
- **Renaming is the risky part.** KayaBot never overwrites a file: a dry run
  shows every result first, and a name that is already taken gets a numbered
  suffix.

```
show.name.s01e02.1080p.web.x264-grp.mkv
        |  parse: "show name", season 1, episode 2
        v
TheTVDB + TVmaze (series) or TMDB + OMDb (movies), queried in parallel
        |  best title, then its episode list
        v
Show Name 1x2 - Episode Title.mkv       preview, dry run, rename
```

## What it does

- Parses file names: `S01E02`, `S01 E02`, `1x02`, `102`, `Season 1 Episode 2`
  (or `Saison`), a release year, and strips quality, codec, audio and
  release-group tags.
- Guesses whether each file is an episode or a movie. You can switch it.
- Matches series against TheTVDB and TVmaze (TMDB on request) and movies
  against TMDB and OMDb. The chosen source and its partner are queried in
  parallel; the "force source" option limits a search to the chosen one.
- Ranks the merged results by source trust, title similarity and the source's
  own rating, then picks the episode by season and number.
- Builds names from templates. The defaults are `{n} {s}x{e} - {t}` for
  episodes and `{n} ({y})` for movies; presets and custom templates are
  available, and a missing value drops with its separator.
- Previews every new name, offers a dry run, and renames in place. It keeps the
  extension, never overwrites a file (a taken name gets ` (1)`, ` (2)` and so
  on), and handles renames that only change the letter case.
- Exports the result of a run as CSV or JSON.

## Installation

### Via Colony (recommended)

Search for **KayaBot** in [Colony](https://github.com/Project-Colony/Colony) and
install it. Updates arrive through the launcher. KayaBot appears there once its
first release is published.

### Direct binary download

No release has been published yet. Each release will carry these assets on the
[latest release](../../releases/latest) page:

| Platform | Asset |
|---|---|
| Linux | `kayabot-linux` |
| Windows | `kayabot-windows.exe` |
| macOS (Apple Silicon) | `kayabot-macos` |
| macOS (Intel) | `kayabot-macos-x86` |

```bash
chmod +x kayabot-linux && ./kayabot-linux
```

### Build from source

```bash
git clone https://github.com/Project-Colony/KayaBot
cd KayaBot
cargo build --release
./target/release/kayabot
```

Requires Rust 1.88 or newer. `kayabot --version` prints the version and exits
without opening a window.

## Configuration

KayaBot keeps its files in one folder:

| Platform | Folder |
|---|---|
| Linux | `~/.config/Colony/KayaBot/` |
| Windows | `%LOCALAPPDATA%\Colony\KayaBot\` |
| macOS | `~/Library/Application Support/Colony/KayaBot/` |

| File | What it holds |
|---|---|
| `api_keys.toml` | your API keys, written by the API configuration section of Settings or by hand |
| `preferences.toml` | the choices made in Settings, saved as you change them |
| `format_options.toml` | the last name template you picked |

TVmaze needs no key. TMDB takes either a v4 read access token or a v3 API key,
and TheTVDB and OMDb each need their own key:

```toml
tmdb_bearer_token = "..."   # or: tmdb_api_key = "..."
tvdb_api_key = "..."
omdb_api_key = "..."
tvmaze_user_agent = "KayaBot"
```

Environment variables take precedence over the file. They are listed in
[`.env.example`](.env.example): `KAYABOT_TMDB_BEARER_TOKEN` (or
`KAYABOT_TMDB_API_KEY`), `KAYABOT_TVDB_API_KEY`, `KAYABOT_OMDB_API_KEY` and
`KAYABOT_TVMAZE_USER_AGENT`.

Older versions also read keys from `config.toml` in the same folder. On start,
KayaBot moves any key it finds there into `api_keys.toml`, unless that file
already sets it, and removes every key from `config.toml`.

## Documentation

- [docs/architecture.md](docs/architecture.md): how the code is laid out and how
  a file goes from its old name to its new one.
- [docs/metadata-sources.md](docs/metadata-sources.md): what each metadata
  source covers, how KayaBot authenticates and what it sends.
- [docs/filebot-parity.md](docs/filebot-parity.md): which FileBot features
  KayaBot has today and which it does not.

## Code signing policy

Every release asset is signed by the Project Colony organisation in CI, never on
a developer machine. Next to each asset on the release page:

| File | What it is |
|---|---|
| `<asset>.sig` | an ed25519 signature over the asset, made with the organisation's release key |
| `<asset>.meta` | three lines binding the asset to its file name, its sha256 and the release version |
| `<asset>.meta.sig` | an ed25519 signature over the `.meta` |

The private key is an organisation secret, used only by the shared
[sign-and-publish workflow](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
in a job that builds nothing; the jobs that compile KayaBot never see it.
Colony checks all three files before it installs or updates KayaBot, and
refuses a release older than the one installed. To check a download yourself
with OpenSSL 3 (the same commands work for every asset):

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=kayabot-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<file name>, sha256=<digest>
sha256sum "$a"    # the digest must equal the sha256 line
```

How releases are built, signed and published:
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing).

<!--
Add the following only once SignPath signs the Windows build, that is once
`signpath-project-slug` is set in .github/workflows/release.yml. Before that it
would promise a signature the .exe does not carry.

Windows releases are also Authenticode-signed. Free code signing provided by
[SignPath.io](https://about.signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

- Committers and reviewers: <GitHub accounts allowed to change the source>
- Approvers: <GitHub accounts that approve each signing request in SignPath>
-->

## Privacy

KayaBot sends no telemetry, no analytics and no crash reports, and it has no
account and no update check.

| Data | Stored or sent | Where, and why |
|---|---|---|
| Settings | stored | `preferences.toml` and `format_options.toml` in the folder under [Configuration](#configuration) |
| API keys | stored | `api_keys.toml` in the same folder, in plain text. On Linux and macOS KayaBot writes it readable by your user only (mode 0600) and tightens an existing file to that when it reads it. Keys given as environment variables are never written to disk |
| Keys left in `config.toml` by older versions | moved | into `api_keys.toml` on start, then removed from `config.toml` |
| Titles to look up | sent | when you match files: the title parsed from the file name, or the one you typed, plus the year for a movie, to the active sources below. The file name, its folder and its contents are never sent, and neither are season or episode numbers: KayaBot downloads the show's episode list and picks the episode locally |
| Title IDs | sent | the ID of the chosen show or movie, to the source that returned it, to fetch its episode list or details |
| Metadata language and region | sent | to TMDB (as `language` and `region`, `fr-FR` when Settings leaves both unset) and to TheTVDB |
| TMDB key | sent | to `api.themoviedb.org` only: a v4 token in the `Authorization` header, a v3 key as the `api_key` parameter in the request URL |
| TheTVDB key | sent | to `api4.thetvdb.com` only, in the body of its login request; the session token it returns is kept in memory and never saved |
| OMDb key | sent | to `www.omdbapi.com` only, as the `apikey` parameter in the request URL |
| User agent | sent | to `api.tvmaze.com`, `KayaBot` unless you set `tvmaze_user_agent`. TVmaze needs no key, so it is queried for series without any configuration |
| Rename summary | stored | only when you export it, as CSV or JSON at the path you choose |
| Your media files | renamed | in place, in their own folder. Nothing is uploaded |

KayaBot keeps no metadata cache: every match asks the sources again. It
connects to no server except the four above, and only when you match files. A
source without a key is skipped without any request. Keys sent in a URL (the
OMDb key and a TMDB v3 key) can appear in the logs of a proxy between you and
the service; a TMDB v4 token avoids that. Like any HTTPS request, each one also
shows the service your IP address.

### Third-party services

Each service handles what it receives under its own policy:

- **TMDB**: [privacy policy](https://www.themoviedb.org/privacy-policy).
  This product uses the TMDB API but is not endorsed or certified by
  [TMDB](https://www.themoviedb.org/)
  ([logo and attribution rules](https://www.themoviedb.org/about/logos-attribution)).
- **TheTVDB**: [privacy policy](https://thetvdb.com/privacy-policy). Metadata
  provided by [TheTVDB](https://thetvdb.com/). Please consider adding missing
  information or subscribing.
- **TVmaze**: [privacy policy](https://www.tvmaze.com/site/privacy). TV data
  from [TVmaze](https://www.tvmaze.com/), licensed under
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
- **OMDb**: [terms](https://www.omdbapi.com/legal.htm). Movie data from the
  [OMDb API](https://www.omdbapi.com/), licensed under
  [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/). OMDb is not
  endorsed by or affiliated with IMDb.com.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
