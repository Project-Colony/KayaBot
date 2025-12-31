# KayaBot

KayaBot est une application desktop Rust (eframe/egui) visant à proposer un outil proche de FileBot pour l'organisation et le renommage de médias.

## Structure du dépôt

- `docs/` : documentation fonctionnelle et technique.
- `tasks/` : suivi des tâches et jalons.
- `src/` : code source Rust.
  - `main.rs` : application eframe/egui (UI de renommage, navigation, état).
  - `formatting/` : génération de chemins de sortie et nettoyage des noms.
  - `matching/` : parsing de fichiers, détection saison/épisode/année.
  - `metadata/` : modèles et fournisseur de métadonnées (stub), cache local.

## Fonctionnalités actuelles

- UI de renommage avec panneaux "Original Files" / "New Names" et barre latérale.
- Parsing heuristique des noms de fichiers (SxxExx, 1x02, année, etc.).
- Prévisualisation des noms via formats par défaut (séries et films).
- Sélecteur de source de métadonnées (enum), fournisseur en mémoire et cache.

## Démarrage rapide

```bash
cargo run
```

## Setup

Créez un fichier de configuration `~/.config/Colony/KayaBot/config.toml` pour stocker les clés API (ou utilisez les variables d’environnement). Les variables d’environnement ont priorité sur le fichier.

Exemple de config :

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

Consultez `docs/docs.md` pour la documentation technique et `tasks/tasks.md` pour le suivi des tâches.
