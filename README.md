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

Consultez `docs/docs.md` pour la documentation technique et `tasks/tasks.md` pour le suivi des tâches.
