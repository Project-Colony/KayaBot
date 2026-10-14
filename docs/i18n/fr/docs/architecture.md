<!-- eidos-i18n: source=docs/architecture.md sha=bb161dc30570220a4ac991fe4c8dbbc08e2b75f4 -->

# Architecture

KayaBot est une seule crate Rust, avec une bibliothèque (`kayabot`,
`src/lib.rs`) et un programme de bureau (`src/main.rs`). La bibliothèque
contient tout, interface comprise ; `main.rs` se contente de répondre à
`--version`, de choisir le dossier de configuration et d'ouvrir la fenêtre.

## Organisation du code

| Chemin | Rôle |
|---|---|
| `src/main.rs` | le point d'entrée : `--version`, le dossier de configuration, puis `ui::run` |
| `src/ui/mod.rs` | `run` : applique les préférences, enregistre les polices et démarre iced |
| `src/ui/app.rs` | `App`, chaque `Message` qu'il traite, le cadre autour des vues, le dépôt de fichiers et les raccourcis clavier |
| `src/ui/sidebar.rs` | la barre latérale : le bouton KayaBot qui ouvre les Préférences, puis l'entrée Renommer |
| `src/ui/preferences.rs` | la page Préférences : Général, Apparence et Accessibilité venant de colony-ui, puis Connexions, Renommage et À propos |
| `src/ui/rename/mod.rs` | l'état et les messages de la vue Renommer, le renommage lui-même et l'aperçu des noms |
| `src/ui/rename/files.rs`, `preview.rs`, `details.rs`, `summary.rs` | la liste des fichiers, les noms proposés avec les options de recherche et de nommage, la fenêtre de correspondance, et le résumé avec son export et la fenêtre de confirmation |
| `src/ui/widgets.rs` | les boutons, cartes, champs et fenêtres propres à KayaBot, stylés d'après la palette, et ses glyphes Nerd Font |
| `src/i18n/` | `t` et `tf`, avec `en.json` et `fr.json` ; le squelette des Préférences et les libellés des thèmes viennent de `colony_ui::i18n` |
| `src/config.rs` | `Config`, enregistré dans `preferences.json`, et la conversion de l'ancien `preferences.toml` |
| `src/api_keys.rs` | les clés venant de l'environnement et de `api_keys.toml`, `write_private_file` et `migrate_config_credentials` |
| `src/lookup.rs` | quelles sources une recherche interroge, ce qu'elle leur demande, et les recherches que l'interface lance hors de son fil d'exécution |
| `src/export.rs` | le résumé du renommage en CSV ou JSON |
| `src/paths.rs` | le dossier de configuration, venant de `colony_ui::paths`, et la copie unique depuis les anciens emplacements |
| `src/matching/parsing.rs` | `parse_filename` : découpe un nom de fichier en jetons et en tire un titre, une saison et un épisode, et une année |
| `src/matching/mod.rs` | `match_files` : transforme chaque nom analysé en un `MatchResult` avec ses candidats (épisode, film ou série), un statut et une confiance |
| `src/matching/advanced.rs` | `normalize_name` et `rank_candidates`, un score Jaro-Winkler et Levenshtein. La chaîne de traitement s'en sert pour la similarité de titre de chaque résultat |
| `src/metadata/provider.rs` | le trait `MetadataProvider` (`search_title`, `fetch_episode_list`, `fetch_movie_details`) et l'énumération `MetadataSource` |
| `src/metadata/providers/` | un client par source : `tmdb.rs`, `thetvdb.rs`, `tvmaze.rs`, `omdb.rs` |
| `src/metadata/aggregate.rs` | `MetadataPipeline` : interroge les sources actives en parallèle, fusionne et classe leurs réponses |
| `src/metadata/models.rs` | `TitleMatch`, `EpisodeMatch`, `MovieMatch` et les `ExternalIds` et informations complémentaires qu'ils portent |
| `src/metadata/locale.rs` | `MetadataLocale`, la langue et la région envoyées aux sources |
| `src/metadata/error.rs` | `MetadataError`, les erreurs qu'un client signale à la fenêtre |
| `src/formatting.rs` | les modèles de noms, le nettoyage des titres et `format_options.toml` |
| `src/rename.rs` | le moteur de renommage |

## L'interface

La fenêtre est construite avec [iced](https://iced.rs) 0.14 et
[colony-ui](https://crates.io/crates/colony-ui), le système de design que
partagent tous les programmes Colony. Les couleurs viennent de la palette
Colony active, le texte des trois polices Colony via `Typography::current()`
(ainsi la police pour la dyslexie et les deux tailles de texte atteignent
chaque widget), et les tailles passent par `sz()`. Les widgets intégrés d'iced
reçoivent la palette via `App::theme`.

`App` suit le modèle d'iced : `update` reçoit un `Message` et modifie l'état,
`view` le dessine. La vue Renommer est un composant à part entière
(`ui::rename::Rename`) avec son propre `Message`, enveloppé dans
`Message::Rename`. Les Préférences remplacent la zone de contenu, jamais une
fenêtre ni une boîte de dialogue, et le bouton KayaBot comme le bouton Fermer
de la page envoient le même `Message::TogglePreferences`. Une modification
dans les Préférences s'applique tout de suite et est enregistrée ; il n'y a
pas de bouton Enregistrer.

## D'un ancien nom à un nouveau

1. **Ajouter des fichiers.** La vue Renommer accepte des fichiers ou un
   dossier choisis dans la boîte de dialogue du système (`rfd`), ou des
   fichiers et dossiers déposés sur la fenêtre.
2. **Analyser.** `matching::match_files` lit chaque nom. `S01E02`, `S01 E02`,
   `1x02`, un nombre à trois chiffres `102` et `Season 1 Episode 2` (ou
   `Saison`) donnent une saison et un épisode ; un nombre à quatre chiffres de
   1900 à 2099 donne une année. Les tags de qualité, de codec, d'audio et de
   groupe de diffusion sont retirés du titre. Une saison et un épisode font du
   fichier un épisode ; une année sans eux en fait un film. Vous pouvez changer
   le type dans la vue.
3. **Choisir les sources.** Par défaut, les séries utilisent TheTVDB avec
   TVmaze pour partenaire, et les films TMDB avec OMDb. La liste active est la
   source choisie plus ses partenaires (TMDB choisi pour une série amène à la
   fois TheTVDB et TVmaze). La case « Utiliser cette source uniquement » ne
   garde que la source choisie.
4. **Chercher.** `MetadataPipeline::search_title` envoie le titre (plus
   l'année pour un film) à toutes les sources actives en même temps, sur des
   threads à portée limitée (scoped threads), et attend qu'elles aient toutes
   répondu. Une source qui échoue ou n'a pas de clé est écartée ; la recherche
   n'échoue que si toutes les sources échouent.
5. **Fusionner et classer.** Les résultats qui partagent un identifiant externe
   (IMDb, TMDB, TheTVDB, TVmaze, OMDb), ou dont les titres sont similaires à au
   moins 0.9 avec des années compatibles, ne forment qu'une entrée. Chaque
   entrée reçoit le score
   `0.55 * trust + 0.35 * title similarity + 0.10 * source rating` (confiance,
   similarité du titre, note de la source), où la confiance vaut 0.9 pour TMDB
   et TheTVDB, 0.75 pour TVmaze et 0.7 pour OMDb.
6. **Épisode.** Pour un épisode, la liste des épisodes du meilleur titre est
   récupérée auprès de la source qui l'a renvoyé, et l'entrée qui porte la
   saison et le numéro analysés est retenue.
7. **Mettre en forme.** `formatting` applique le modèle à chaque fichier (voir
   plus bas).
8. **Aperçu, simulation, renommage.** `rename::apply` construit la cible à
   côté de l'original et conserve l'extension. En mode simulation, il résout le
   nom et ne touche à rien ; sinon, une fois que l'utilisateur a confirmé (sauf
   si c'est désactivé dans les Préférences), il renomme, puis la liste se
   recharge à partir des nouveaux noms. Le résultat de chaque fichier alimente
   le résumé, qui peut être exporté.

Les clients des sources utilisent du HTTP bloquant avec un délai d'expiration
de 10 secondes. L'interface ne les appelle jamais elle-même : `Rename::run`
confie la recherche à un thread de travail et en reçoit la réponse sous forme
de message, et Associer et Rechercher restent désactivés pendant ce temps. Une
association lancée avant que la liste de fichiers ne change est abandonnée à
son arrivée.

## Modèles de noms

| Jeton | Valeur |
|---|---|
| `{n}` | le titre de la série ou du film (aussi `{name}`, `{title}`) |
| `{t}` | le titre de l'épisode (aussi `{episodetitle}`) |
| `{s}` | le numéro de saison (aussi `{season}`) |
| `{e}` | le numéro d'épisode (aussi `{episode}`) |
| `{y}` | l'année (aussi `{year}`) |

Des chiffres après un jeton complètent le nombre par des zéros : `{s00}` donne
`01`. Les modèles par défaut sont `{n} {s}x{e} - {t}` pour les épisodes et
`{n} ({y})` pour les films ; désactiver le titre de l'épisode ou l'année dans
la vue les fait passer à `{n} {s}x{e}` et `{n}`. Quand un jeton n'a pas de
valeur, il disparaît avec les séparateurs qui l'entourent : un film sans année
devient `Title`, pas `Title ()`. Les titres sont nettoyés avant usage : tout
caractère qui n'est ni une lettre ni un chiffre devient une espace (ainsi
`Grey's` devient `Grey s`, et aucun caractère interdit dans un nom de fichier
ne subsiste), les tags techniques (`1080p`, `BluRay`, `VF`, `VOSTFR`, etc.)
sont supprimés, et les espaces multiples sont réduites à une seule.

## Moteur de renommage

`src/rename.rs` construit le chemin cible à côté de l'original, choisit le
premier nom libre (` (1)`, ` (2)` et ainsi de suite, jusqu'à 9999) et renomme
le fichier. Il n'écrase jamais un fichier existant, y compris un lien physique
vers l'original. Deux fichiers d'un même lot ne reçoivent jamais des noms qui
ne diffèrent que par la casse. Un renommage qui ne change que la casse passe
par un nom temporaire ; un déplacement entre deux périphériques copie dans un
nouveau fichier, le synchronise sur le disque, puis supprime l'original.

## Fichiers de configuration

`paths::config_dir` vaut `colony_ui::paths::config_dir("KayaBot")` :
`~/.config/Colony/KayaBot` sous Linux, `%LOCALAPPDATA%\Colony\KayaBot` sous
Windows et `~/Library/Application Support/Colony/KayaBot` sous macOS.

Avant toute lecture, `paths::init` vérifie ce dossier. S'il est vide et qu'un
ancien emplacement contient des fichiers (d'abord le profil itinérant de
Windows, `%APPDATA%\Colony\KayaBot`, puis `~/.kayabot`), il les copie et écrit
un marqueur `.migrated`. L'ancien dossier n'est jamais modifié. Une copie qui
échoue est annulée, et l'ancien dossier est utilisé pour cette exécution.

- `preferences.json` : `Config`, c'est-à-dire les `StandardPreferences` de
  colony-ui (thème, couleur d'accent, tailles de texte, contraste élevé,
  police pour la dyslexie, animations réduites), plus la langue de
  l'interface, la langue et la région des métadonnées, les deux confirmations
  et le mode simulation par défaut. Chaque champ est lu séparément : une valeur
  invalide ne réinitialise que ce champ. Écrit à chaque changement de
  préférence. S'il manque, le `preferences.toml` d'une ancienne version, à plat
  ou en sections, est lu une fois et enregistré au nouveau format ; ses
  réglages qui n'ont jamais rien fait sont abandonnés.
- `api_keys.toml` : lu au démarrage ; écrit au fil de la saisie des clés dans
  Préférences, Connexions, via `write_private_file`, qui écrit un fichier
  temporaire avec le mode 0600 sous Unix et le renomme à sa place. Les
  variables d'environnement priment sur chaque clé.
- `format_options.toml` : les modèles de noms et les choix concernant l'année
  et le titre de l'épisode.
- `config.toml` : seulement pour les anciennes versions. Avant l'ouverture de
  la fenêtre, `migrate_config_credentials` déplace ses clés dans
  `api_keys.toml` et les en retire, identifiants AniDB compris.

## Tests

- Les tests unitaires se trouvent à côté du code : les modèles, le moteur de
  renommage, la chaîne de traitement, chaque client de source face aux
  réponses enregistrées dans `tests/fixtures/`, la migration des clés et les
  écritures de fichiers privés, l'export, les préférences et leur conversion,
  la copie depuis les anciens dossiers, et la fonction `update` de l'interface
  avec une source simulée (`ui/app.rs`, `ui/rename/tests.rs`). Les tests
  unitaires écrivent dans un dossier temporaire, jamais dans le vrai profil.
- `i18n` vérifie que `en.json` et `fr.json` ont les mêmes clés et les mêmes
  paramètres, et que chaque clé demandée par l'interface existe ; `ui/widgets.rs`
  vérifie que chaque glyphe existe dans la Nerd Font.
- `tests/matching_snapshots.rs` passe l'analyseur sur un épisode, un film et
  un titre seul et compare le résultat à un instantané `insta`. Les fichiers
  vides de `tests/fixtures/media_samples` alimentent les tests du renommage et
  de la vue Renommer.
- `tests/integration_api.rs` appelle les vrais services. Ses tests sont
  ignorés par défaut et ont besoin des clés de `.env.example` :

  ```bash
  KAYABOT_TMDB_API_KEY=... KAYABOT_OMDB_API_KEY=... KAYABOT_TVDB_API_KEY=... \
    cargo test --test integration_api -- --ignored
  ```

  `KAYABOT_TMDB_BEARER_TOKEN` fonctionne à la place de `KAYABOT_TMDB_API_KEY`.
