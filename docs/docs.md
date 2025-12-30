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

#### Attributs attendus par source (films / séries)

Les attributs ci-dessous correspondent aux champs utilisés par KayaBot (modèles `TitleMatch` et `EpisodeMatch`).

| Source | Films (Title, Year) | Séries (Series Title, Season, Episode, Episode Title) |
| --- | --- | --- |
| TheMovieDB | Title, Year | Series Title, Season, Episode, Episode Title |
| AniDB | Title | Series Title, Season, Episode, Episode Title |
| TheTVDB | — | Series Title, Season, Episode, Episode Title |
| TVmaze | — | Series Title, Season, Episode, Episode Title |

Remarques :
- Les films n'utilisent pas AniDB/TheTVDB/TVmaze dans l'UI actuelle, mais la liste ci-dessus fixe les champs attendus si l'intégration est ajoutée.
- Tous les champs sont optionnels côté UI : un `Year` ou un `Episode Title` manquant déclenche un fallback de formatage.

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
