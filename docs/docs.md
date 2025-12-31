# Documentation

Ce dossier rassemble la documentation fonctionnelle et technique de KayaBot.

## Objectif

- Décrire les fonctionnalités principales (organisation et renommage de médias, comme FileBot).
- Préciser les sources de métadonnées et les règles de correspondance.
- Détailler les conventions de configuration, logs, et formats de sortie.

## Architecture actuelle (code)

### UI (eframe/egui)

- `src/main.rs` porte l'application eframe/egui et l'état principal (`RenameApp`).
- L'interface actuelle propose une vue "Rename" avec deux listes (fichiers d'origine / nouveaux noms), un panneau d'action (Match/Rename) et un menu de sélection de source de métadonnées.

### Matching & parsing

- `src/matching/parsing.rs` implémente un parseur heuristique (SxxExx, 1x02, année, tokens).
- `src/matching/mod.rs` calcule un `MatchResult` (statut, confiance, candidats) pour chaque fichier.

### Formatting

- `src/formatting/mod.rs` génère les formats finaux pour séries et films, applique les fallbacks et normalise les titres.
- Les formats par défaut sont disponibles via `DEFAULT_SERIES_FORMAT` et `DEFAULT_MOVIE_FORMAT`.
- Normalisation des titres :
  - suppression des tags techniques (VF, 1080p, BluRay, etc.),
  - remplacement de la ponctuation par des espaces,
  - réduction des espaces multiples.

Templates FileBot (tokens supportés) :

- `{n}` : titre principal (série ou film).
- `{t}` : titre d'épisode.
- `{s}` / `{s00}` : numéro de saison (avec padding optionnel).
- `{e}` / `{e00}` : numéro d'épisode (avec padding optionnel).
- `{y}` : année de sortie (si disponible).

Formats finaux (et fallbacks) :

- Films : `{n} ({y})` (si `{y}` est absent, la parenthèse est supprimée).
- Séries : `{n} {s}x{e} - {t}` (si `{t}` est absent, le suffixe ` - {t}` est supprimé).

### Métadonnées

- `src/metadata/provider.rs` définit l'interface `MetadataProvider` et l'énumération `MetadataSource`.
- `src/metadata/aggregate.rs` implémente `MetadataPipeline`, qui orchestre plusieurs providers et agrège les résultats.
- Les clients réels sont dans `src/metadata/providers/` (`tmdb.rs`, `thetvdb.rs`, `tvmaze.rs`, `omdb.rs`, `anidb.rs`).
- `src/metadata/filebot_like.rs` reste un provider en mémoire avec cache local (`MetadataCache`), utile pour le mock et l'outillage.
- `src/metadata/models.rs` définit les types `TitleMatch`, `MovieMatch`, `EpisodeMatch` ainsi que les modèles normalisés (`NormalizedTitle`, `NormalizedEpisode`).

#### Stratégie de sélection des sources

- Séries : TheTVDB en source primaire, TVmaze en fallback si aucune réponse.
- Films : TheMovieDB (TMDB) en source primaire, OMDb en fallback si aucune réponse.
- L'app peut forcer une source unique (option "Forcer la source"), sinon elle conserve une liste active (source principale + fallbacks).

#### Attributs attendus par source (normalisés)

Les tableaux ci-dessous listent les champs requis vs optionnels pour alimenter les formats cibles (modèles `NormalizedTitle` et `NormalizedEpisode`).

##### Films (format cible : `Title`, `Release Year`, `IMDb ID`, extras)

| Source | Requis | Optionnels |
| --- | --- | --- |
| TheMovieDB | `title`, `source_id` | `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |
| OMDb | `title`, `source_id` | `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |
| AniDB | `title`, `source_id` | `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |

##### Séries / épisodes (format cible : `Series Title`, `Season`, `Episode`, `Episode Title`, `Release Year`, `IMDb ID`, extras)

| Source | Requis | Optionnels |
| --- | --- | --- |
| TheTVDB | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |
| TVmaze | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |
| AniDB | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id`, `extras.aliases`, `extras.language`, `extras.genres`, `extras.external_ids`, `extras.synopsis` |

Remarques :
- Les films n'utilisent pas AniDB dans l'UI actuelle, mais la table fixe les champs attendus si l'intégration est ajoutée.
- Les champs optionnels peuvent déclencher des fallbacks de formatage (ex: absence de `release_year` ou `episode_title`).

#### Mapping source → modèle interne (normalisé)

Ce mapping décrit les correspondances minimales à appliquer lors de l'implémentation des adapters d'API.

##### TheMovieDB (film)
- `id` → `source_id`
- `title` → `title`
- `release_date` → `release_year` (année extraite)
- `imdb_id` → `imdb_id`
- `original_title`/`also_known_as` → `extras.aliases`
- `original_language` → `extras.language`
- `genre_ids`/`genres[].name` → `extras.genres`
- `external_ids.*` → `extras.external_ids`
- `overview` → `extras.synopsis`
- `vote_average` → `source_score`, combiné avec `source_trust` pour `global_score`

##### OMDb (film)
- `imdbID` → `imdb_id` (et `source_id` si OMDb est la source primaire)
- `Title` → `title`
- `Year` → `release_year`
- `Language` → `extras.language`
- `Genre` → `extras.genres` (liste scindée par virgule)
- `Plot` → `extras.synopsis`
- `Ratings[]` → `source_score` (normalisé), combiné avec `source_trust` pour `global_score`

##### AniDB (film/série)
- `anime_id` → `source_id` / `series_id`
- `title` → `title` / `series_title`
- `year` → `release_year`
- `aliases` → `extras.aliases`
- `language` → `extras.language`
- `tags`/`genres` → `extras.genres`
- `description` → `extras.synopsis`

##### TheTVDB (série/épisode)
- `series.id` → `series_id`
- `series.name` → `series_title`
- `episode.seasonNumber` → `season`
- `episode.number` → `episode`
- `episode.name` → `episode_title`
- `episode.year` → `release_year`
- `series.aliases[]` → `extras.aliases`
- `series.language` → `extras.language`
- `series.genres[]` → `extras.genres`
- `series.overview` → `extras.synopsis`
- `series.id`/`episode.id` → `extras.external_ids`
- `series.score` → `source_score`, combiné avec `source_trust` pour `global_score`

##### TVmaze (série/épisode)
- `show.id` → `series_id`
- `show.name` → `series_title`
- `season.number` → `season`
- `episode.number` → `episode`
- `episode.name` → `episode_title`
- `episode.airdate` → `release_year` (année extraite)
- `show.language` → `extras.language`
- `show.genres[]` → `extras.genres`
- `show.summary` → `extras.synopsis`
- `show.externals.*` → `extras.external_ids`
- `score` → `source_score`, combiné avec `source_trust` pour `global_score`

## Flux utilisateur (prototype)

1. L'utilisateur charge une liste de fichiers (UI en cours d'intégration).
2. "Match" applique le parsing heuristique et alimente les suggestions.
3. "Fetch Data" interroge les providers actifs (TMDB/TheTVDB/TVmaze/OMDb/AniDB).
4. "Rename" simule le renommage et affiche un bilan.

## Tests & validation

- Des tests unitaires existent pour le formatage (voir `src/formatting/mod.rs`).
- Étendre la couverture au parsing/matching reste prioritaire pour stabiliser le MVP.

## Configuration

Les clés API peuvent être définies dans le fichier
`~/.config/Colony/KayaBot/api_keys.toml` ou via des variables d’environnement
(les variables d’environnement ont priorité sur les fichiers).

Le fichier `config.toml` est utilisé par l'application pour persister la file
et les options de formatage. `preferences.toml` conserve les préférences UI
(thème, langue/locale, densité, options d'expérience). `format_options.toml`
contient le dernier template de formatage choisi.

Exemple de configuration :

```toml
tmdb_bearer_token = "..."
tvdb_api_key = "..."
omdb_api_key = "..."
anidb_api_key = "..."
tvmaze_user_agent = "KayaBot"
```

Variables d’environnement supportées :

- `KAYABOT_TMDB_BEARER_TOKEN` (ou `KAYABOT_TMDB_API_KEY`)
- `KAYABOT_TVDB_API_KEY`
- `KAYABOT_OMDB_API_KEY`
- `KAYABOT_ANIDB_PASSWORD` (ou `KAYABOT_ANIDB_API_KEY`)
- `KAYABOT_TVMAZE_USER_AGENT` (ou `KAYABOT_TVMAZE_API_KEY`)
