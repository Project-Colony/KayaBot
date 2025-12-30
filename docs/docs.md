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

Formats finaux (et fallbacks) :

- Films : `{Title} ({Year})` (si `Year` est absent, on affiche uniquement `{Title}`).
- Séries : `{Series Title} {Season}x{Episode} - {Episode Title}` (si `Episode Title` est absent, le suffixe ` - {Episode Title}` est omis).

### Métadonnées

- `src/metadata/provider.rs` définit l'interface `MetadataProvider` et l'énumération `MetadataSource`.
- `src/metadata/filebot_like.rs` fournit un provider en mémoire (dataset vide par défaut) et un cache local (`MetadataCache`).
- `src/metadata/models.rs` définit les types `TitleMatch` et `EpisodeMatch`.

#### Stratégie de sélection des sources

- Séries : TheTVDB en source primaire, TVmaze en fallback si aucune réponse.
- Films : TheMovieDB (TMDB) en source primaire, OMDb en fallback si aucune réponse.

#### Attributs attendus par source (normalisés)

Les tableaux ci-dessous listent les champs requis vs optionnels pour alimenter les formats cibles (modèles `NormalizedTitle` et `NormalizedEpisode`).

##### Films (format cible : `Title`, `Release Year`, `IMDb ID`)

| Source | Requis | Optionnels |
| --- | --- | --- |
| TheMovieDB | `title`, `source_id` | `release_year`, `imdb_id` |
| OMDb | `title`, `source_id` | `release_year`, `imdb_id` |
| AniDB | `title`, `source_id` | `release_year`, `imdb_id` |

##### Séries / épisodes (format cible : `Series Title`, `Season`, `Episode`, `Episode Title`, `Release Year`, `IMDb ID`)

| Source | Requis | Optionnels |
| --- | --- | --- |
| TheTVDB | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id` |
| TVmaze | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id` |
| AniDB | `series_title`, `series_id`, `season`, `episode` | `episode_title`, `release_year`, `imdb_id` |

Remarques :
- Les films n'utilisent pas AniDB dans l'UI actuelle, mais la table fixe les champs attendus si l'intégration est ajoutée.
- Les champs optionnels peuvent déclencher des fallbacks de formatage (ex: absence de `release_year` ou `episode_title`).

#### Mapping source → format cible (normalisé)

Ce mapping décrit les correspondances minimales à appliquer lors de l'implémentation des adapters d'API.

##### TheMovieDB (film)
- `id` → `source_id`
- `title` → `title`
- `release_date` → `release_year` (année extraite)
- `imdb_id` → `imdb_id`

##### OMDb (film)
- `imdbID` → `imdb_id` (et `source_id` si OMDb est la source primaire)
- `Title` → `title`
- `Year` → `release_year`

##### AniDB (film/série)
- `anime_id` → `source_id` / `series_id`
- `title` → `title` / `series_title`
- `year` → `release_year`

##### TheTVDB (série/épisode)
- `series.id` → `series_id`
- `series.name` → `series_title`
- `episode.seasonNumber` → `season`
- `episode.number` → `episode`
- `episode.name` → `episode_title`
- `episode.year` → `release_year`

##### TVmaze (série/épisode)
- `show.id` → `series_id`
- `show.name` → `series_title`
- `season.number` → `season`
- `episode.number` → `episode`
- `episode.name` → `episode_title`
- `episode.airdate` → `release_year` (année extraite)

## Flux utilisateur (prototype)

1. L'utilisateur charge une liste de fichiers (UI en cours d'intégration).
2. "Match" applique le parsing heuristique et alimente les suggestions.
3. "Fetch Data" interroge le provider de métadonnées (actuellement stub).
4. "Rename" simule le renommage et affiche un bilan.

## Tests & validation

- Aucun test automatisé n'est encore défini.
- Ajouter des tests unitaires sur le parsing/formatage est prioritaire pour stabiliser le MVP.

## Configuration (placeholders)

Les clés d'API et identifiants sont documentés via `.env.example` (à copier en `.env` si besoin). Aucune lecture d'env n'est encore implémentée côté code, mais les variables suivantes sont prévues :

- `KAYABOT_TMDB_API_KEY`
- `KAYABOT_TMDB_BEARER_TOKEN`
- `KAYABOT_TVDB_API_KEY`
- `KAYABOT_TVDB_PIN`
- `KAYABOT_TVMAZE_API_KEY`
- `KAYABOT_OMDB_API_KEY`
- `KAYABOT_ANIDB_USERNAME`
- `KAYABOT_ANIDB_PASSWORD`
- `KAYABOT_ANIDB_CLIENT_NAME`
- `KAYABOT_ANIDB_CLIENT_VERSION`
