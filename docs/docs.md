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

- `src/formatting/mod.rs` génère les chemins par défaut pour séries et films et sanitise les composants.
- Les formats par défaut sont disponibles via `DEFAULT_SERIES_FORMAT` et `DEFAULT_MOVIE_FORMAT`.

### Métadonnées

- `src/metadata/provider.rs` définit l'interface `MetadataProvider` et l'énumération `MetadataSource`.
- `src/metadata/filebot_like.rs` fournit un provider en mémoire (dataset vide par défaut) et un cache local (`MetadataCache`).
- `src/metadata/models.rs` définit les types `TitleMatch` et `EpisodeMatch`.

## Flux utilisateur (prototype)

1. L'utilisateur charge une liste de fichiers (UI en cours d'intégration).
2. "Match" applique le parsing heuristique et alimente les suggestions.
3. "Fetch Data" interroge le provider de métadonnées (actuellement stub).
4. "Rename" simule le renommage et affiche un bilan.

## Tests & validation

- Aucun test automatisé n'est encore défini.
- Ajouter des tests unitaires sur le parsing/formatage est prioritaire pour stabiliser le MVP.
