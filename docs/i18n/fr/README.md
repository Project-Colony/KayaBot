<!-- eidos-i18n: source=README.md sha=eda50a54468479b46b52f47c8ceb98dec5a37c22 -->

<div align="center">

**Un renommeur de médias de bureau pour les bibliothèques de films et de séries : il cherche vos fichiers dans TMDB, TheTVDB, TVmaze et OMDb et leur donne des noms propres et cohérents.**

</div>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](../../../LICENSE)
[![Colony app](https://img.shields.io/badge/Colony-multimedia-purple)](https://github.com/Project-Colony/Colony)
[![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20windows%20%7C%20macOS-lightgrey)](#installation)

Les épisodes et films téléchargés arrivent avec des noms comme
`show.name.s01e02.1080p.web.x264-grp.mkv`. Un renommeur de masse peut
nettoyer la ponctuation, mais il ne connaît pas le titre de l'épisode, et
FileBot, l'outil habituel pour ce travail, demande une licence payante.
KayaBot analyse les noms de fichiers, interroge plusieurs bases de données en
ligne à la fois et affiche chaque nouveau nom avant de toucher au moindre
fichier.

> **État :** prototype précoce, et aucune version n'a encore été publiée. La
> vue Renommer fonctionne de bout en bout : analyse, correspondance avec TMDB,
> TheTVDB, TVmaze et OMDb, aperçu, simulation avec une confirmation avant un
> vrai renommage, le renommage lui-même et un résumé en CSV ou JSON.
> L'interface est construite avec iced et le système de design Colony, en
> anglais et en français ; les Préférences regroupent le thème,
> l'accessibilité, la langue, les clés d'API et les options de renommage.
> L'analyseur, les modèles de noms, le moteur de renommage, les quatre clients
> de sources et la gestion d'état de l'interface ont des tests automatisés,
> les clients face à des réponses enregistrées ; l'apparence de la fenêtre
> n'est vérifiée qu'à la main.

## Pourquoi KayaBot

- **Un renommeur de masse ou une regex** range la ponctuation, mais vous
  tapez encore chaque titre d'épisode. KayaBot récupère les titres et l'année.
- **FileBot** demande une licence payante. KayaBot est un logiciel libre sous
  GPL-3.0-or-later.
- **Aucune base de données n'est jamais complète.** KayaBot interroge
  plusieurs sources en parallèle et fusionne leurs réponses par identifiants
  communs (IMDb, TMDB, TheTVDB, TVmaze) et similarité des titres : une série
  absente d'une source est quand même trouvée.
- **Le renommage est l'étape risquée.** KayaBot n'écrase jamais un fichier :
  une simulation montre d'abord chaque résultat, et un nom déjà pris reçoit un
  suffixe numéroté.

```
show.name.s01e02.1080p.web.x264-grp.mkv
        |  analyse : "show name", saison 1, épisode 2
        v
TheTVDB + TVmaze (séries) ou TMDB + OMDb (films), interrogés en parallèle
        |  meilleur titre, puis sa liste d'épisodes
        v
Show Name 1x2 - Episode Title.mkv       aperçu, simulation, renommage
```

## Ce qu'il fait

- Analyse les noms de fichiers : `S01E02`, `S01 E02`, `1x02`, `102`,
  `Season 1 Episode 2` (ou `Saison`), une année de sortie, et retire les tags
  de qualité, de codec, d'audio et de groupe de diffusion.
- Devine si chaque fichier est un épisode ou un film. Vous pouvez changer le
  type.
- Cherche les séries dans TheTVDB et TVmaze (TMDB sur demande) et les films
  dans TMDB et OMDb. La source choisie et son partenaire sont interrogés en
  parallèle ; l'option « Utiliser cette source uniquement » limite une
  recherche à la source choisie. Les recherches tournent en arrière-plan : la
  fenêtre reste réactive.
- Classe les résultats fusionnés selon la confiance accordée à la source, la
  similarité des titres et la note propre à la source, puis choisit l'épisode
  par saison et numéro.
- Construit les noms à partir de modèles. Par défaut, `{n} {s}x{e} - {t}` pour
  les épisodes et `{n} ({y})` pour les films ; des modèles prédéfinis et
  personnalisés sont disponibles, et une valeur manquante disparaît avec son
  séparateur.
- Affiche chaque nouveau nom, propose une simulation, demande confirmation
  avant un vrai renommage (vous pouvez le désactiver) et renomme sur place. Il
  conserve l'extension, n'écrase jamais un fichier (un nom pris reçoit ` (1)`,
  ` (2)` et ainsi de suite) et gère les renommages qui ne changent que la
  casse.
- Exporte le résultat d'un passage en CSV ou en JSON.

## Installation

### Via Colony (recommandé)

Cherchez **KayaBot** dans [Colony](https://github.com/Project-Colony/Colony) et
installez-le. Les mises à jour arrivent par le lanceur. KayaBot y apparaîtra
dès la publication de sa première version.

### Téléchargement direct du binaire

Aucune version n'a encore été publiée. Chaque version portera ces fichiers sur
la page de la [dernière version](https://github.com/Project-Colony/KayaBot/releases/latest) :

| Plateforme | Fichier |
|---|---|
| Linux | `kayabot-linux` |
| Windows | `kayabot-windows.exe` |
| macOS (Apple Silicon) | `kayabot-macos` |
| macOS (Intel) | `kayabot-macos-x86` |

```bash
chmod +x kayabot-linux && ./kayabot-linux
```

### Compiler depuis les sources

```bash
git clone https://github.com/Project-Colony/KayaBot
cd KayaBot
cargo build --release
./target/release/kayabot
```

Nécessite Rust 1.88 ou plus récent. `kayabot --version` affiche la version et
quitte sans ouvrir de fenêtre.

## Configuration

KayaBot garde ses fichiers dans un seul dossier :

| Plateforme | Dossier |
|---|---|
| Linux | `~/.config/Colony/KayaBot/` |
| Windows | `%LOCALAPPDATA%\Colony\KayaBot\` |
| macOS | `~/Library/Application Support/Colony/KayaBot/` |

| Fichier | Contenu |
|---|---|
| `preferences.json` | les choix faits dans les Préférences, enregistrés au fur et à mesure |
| `api_keys.toml` | vos clés d'API, saisies dans les Préférences, rubrique Connexions, ou écrites à la main |
| `format_options.toml` | les modèles de noms, et si les noms portent l'année et le titre de l'épisode |

Les anciennes versions gardaient ces fichiers dans `~/.kayabot/` et, sous
Windows, dans le profil itinérant, `%APPDATA%\Colony\KayaBot\`. Tant que le
dossier ci-dessus est vide, le premier démarrage les y copie et laisse un
marqueur `.migrated`. L'ancien dossier reste tel quel, à supprimer à la main ;
si la copie échoue, KayaBot continue d'utiliser l'ancien dossier et réessaie
au démarrage suivant. Un `preferences.toml` d'une ancienne version est lu une
fois et enregistré en `preferences.json`.

TVmaze n'a pas besoin de clé. TMDB accepte soit un jeton d'accès en lecture
v4, soit une clé API v3, et TheTVDB et OMDb ont chacun besoin de leur propre
clé :

```toml
tmdb_bearer_token = "..."   # ou : tmdb_api_key = "..."
tvdb_api_key = "..."
omdb_api_key = "..."
tvmaze_user_agent = "KayaBot"
```

Les variables d'environnement ont priorité sur le fichier. Elles sont listées
dans [`.env.example`](../../../.env.example) : `KAYABOT_TMDB_BEARER_TOKEN` (ou
`KAYABOT_TMDB_API_KEY`), `KAYABOT_TVDB_API_KEY`, `KAYABOT_OMDB_API_KEY` et
`KAYABOT_TVMAZE_USER_AGENT`. `KAYABOT_TVMAZE_API_KEY` est un ancien nom de
l'agent utilisateur TVmaze, lu seulement quand `KAYABOT_TVMAZE_USER_AGENT`
n'est pas défini ; TVmaze n'a pas de clé d'API.

Les anciennes versions lisaient aussi les clés dans `config.toml`, dans le
même dossier. Au démarrage, KayaBot déplace chaque clé qu'il y trouve vers
`api_keys.toml`, sauf si ce fichier la définit déjà, et retire toutes les clés
de `config.toml`.

## Documentation

- [docs/architecture.md](docs/architecture.md) : l'organisation du code et le
  chemin d'un fichier de son ancien nom à son nouveau nom.
- [docs/metadata-sources.md](docs/metadata-sources.md) : ce que couvre chaque
  source de métadonnées, comment KayaBot s'authentifie et ce qu'il envoie.
- [docs/filebot-parity.md](docs/filebot-parity.md) : les fonctionnalités de
  FileBot que KayaBot a aujourd'hui et celles qu'il n'a pas.

## Langue

Ce README et les pages ci-dessus existent aussi en français. **L'anglais fait
foi** : quand une traduction le contredit, c'est le fichier anglais qui a
raison.

- **Français** - [README](README.md) · [architecture](docs/architecture.md) · [sources de métadonnées](docs/metadata-sources.md) · [parité avec FileBot](docs/filebot-parity.md)

Chaque traduction porte l'empreinte du fichier anglais dont elle est issue, et
la CI échoue quand l'anglais avance : voir [`scripts/i18n-check.sh`](../../../scripts/i18n-check.sh).
`docs/i18n/fr/` reproduit la racine du dépôt : `docs/i18n/fr/docs/architecture.md`
est la version française de `docs/architecture.md`.

## Politique de signature du code

Chaque fichier de version est signé par l'organisation Project Colony dans la
CI, jamais sur la machine d'un développeur. À côté de chaque fichier, sur la
page de la version :

| Fichier | Ce que c'est |
|---|---|
| `<asset>.sig` | une signature ed25519 du fichier, faite avec la clé de publication de l'organisation |
| `<asset>.meta` | trois lignes qui lient le fichier à son nom, à son sha256 et au numéro de version |
| `<asset>.meta.sig` | une signature ed25519 du `.meta` |

La clé privée est un secret de l'organisation, utilisé uniquement par le
[workflow sign-and-publish](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/.github/workflows/sign-and-publish.yml)
partagé, dans un job qui ne compile rien ; les jobs qui compilent KayaBot ne
la voient jamais. Colony vérifie les trois fichiers avant d'installer ou de
mettre à jour KayaBot, et refuse une version plus ancienne que celle qui est
installée. Pour vérifier vous-même un téléchargement avec OpenSSL 3 (les mêmes
commandes valent pour chaque fichier) :

```bash
cat > colony-release.pub <<'EOF'
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARNjg3Nn8H6/aBg1unwGjkUTcrdTxERNefVaqU8cFu0s=
-----END PUBLIC KEY-----
EOF
a=kayabot-linux
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a" -sigfile "$a.sig"
openssl pkeyutl -verify -pubin -inkey colony-release.pub -rawin -in "$a.meta" -sigfile "$a.meta.sig"
cat "$a.meta"     # version=<tag>, asset=<nom du fichier>, sha256=<condensé>
sha256sum "$a"    # le condensé doit être égal à la ligne sha256
```

Comment les versions sont compilées, signées et publiées :
[design/releases.md](https://github.com/Project-Colony/Project-Colony-Resources/blob/main/design/releases.md#5-signing)
(en anglais).

<!--
N'ajouter ce qui suit qu'une fois que SignPath signe la version Windows,
c'est-à-dire une fois que `signpath-project-slug` est défini dans
.github/workflows/release.yml. Avant cela, ce texte promettrait une signature
que le .exe ne porte pas.

Les versions Windows sont aussi signées Authenticode. Signature de code
gratuite fournie par [SignPath.io](https://about.signpath.io/), certificat
fourni par la [SignPath Foundation](https://signpath.org/).

- Contributeurs et relecteurs : <comptes GitHub autorisés à modifier les sources>
- Approbateurs : <comptes GitHub qui approuvent chaque demande de signature dans SignPath>
-->

## Confidentialité

KayaBot n'envoie ni télémétrie, ni statistiques d'usage, ni rapports de
plantage, et il n'a ni compte ni vérification de mises à jour.

| Donnée | Conservée ou envoyée | Où, et pourquoi |
|---|---|---|
| Préférences | conservées | `preferences.json` et `format_options.toml` dans le dossier indiqué sous [Configuration](#configuration) |
| Fichiers des anciennes versions | copiés | une fois, depuis `~/.kayabot/` ou le profil itinérant de Windows vers le dossier indiqué sous [Configuration](#configuration). Rien ne quitte l'ordinateur, et les anciennes copies, clés d'API comprises, restent où elles étaient jusqu'à ce que vous les supprimiez |
| Clés d'API | conservées | `api_keys.toml` dans le même dossier, en clair. Sous Linux et macOS, KayaBot l'écrit lisible par votre seul utilisateur (mode 0600) et resserre à ce mode un fichier existant quand il le lit. Les clés fournies par variables d'environnement ne sont jamais écrites sur le disque |
| Clés laissées dans `config.toml` par les anciennes versions | déplacées | vers `api_keys.toml` au démarrage, puis retirées de `config.toml` |
| Titres à rechercher | envoyés | quand vous lancez une association : le titre tiré du nom de fichier, ou celui que vous avez saisi, plus l'année pour un film, aux sources actives ci-dessous. Le nom du fichier, son dossier et son contenu ne sont jamais envoyés, pas plus que les numéros de saison ou d'épisode : KayaBot télécharge la liste des épisodes de la série et choisit l'épisode localement |
| Identifiants de titres | envoyés | l'identifiant de la série ou du film choisi, à la source qui l'a renvoyé, pour récupérer sa liste d'épisodes ou ses détails |
| Langue et région des métadonnées | envoyées | à TMDB (en `language` et `region`, `fr-FR` quand les Préférences laissent les deux vides) et à TheTVDB |
| Clé TMDB | envoyée | à `api.themoviedb.org` uniquement : un jeton v4 dans l'en-tête `Authorization`, une clé v3 en paramètre `api_key` dans l'URL de la requête |
| Clé TheTVDB | envoyée | à `api4.thetvdb.com` uniquement, dans le corps de sa requête de connexion ; le jeton de session qu'il renvoie est gardé en mémoire et jamais enregistré |
| Clé OMDb | envoyée | à `www.omdbapi.com` uniquement, en paramètre `apikey` dans l'URL de la requête |
| Agent utilisateur | envoyé | à `api.tvmaze.com`, `KayaBot` sauf si vous définissez `tvmaze_user_agent`. TVmaze n'a pas besoin de clé : il est donc interrogé pour les séries sans aucune configuration |
| Résumé du renommage | conservé | seulement quand vous l'exportez, en CSV ou JSON, au chemin que vous choisissez |
| Vos fichiers médias | renommés | sur place, dans leur propre dossier. Rien n'est téléversé |

KayaBot ne garde aucun cache de métadonnées : chaque association interroge de
nouveau les sources. Il ne se connecte à aucun serveur en dehors des quatre
ci-dessus, et seulement quand vous lancez une association. Une source sans clé
est ignorée sans aucune requête. Les clés envoyées dans une URL (la clé OMDb
et une clé TMDB v3) peuvent apparaître dans les journaux d'un proxy situé
entre vous et le service ; un jeton TMDB v4 évite cela. Comme toute requête
HTTPS, chacune montre aussi votre adresse IP au service.

### Services tiers

Chaque service traite ce qu'il reçoit selon sa propre politique :

- **TMDB** : [politique de confidentialité](https://www.themoviedb.org/privacy-policy).
  Ce produit utilise l'API TMDB mais n'est ni approuvé ni certifié par
  [TMDB](https://www.themoviedb.org/)
  ([règles de logo et d'attribution](https://www.themoviedb.org/about/logos-attribution)).
- **TheTVDB** : [politique de confidentialité](https://thetvdb.com/privacy-policy).
  Métadonnées fournies par [TheTVDB](https://thetvdb.com/). Pensez à y ajouter
  les informations manquantes ou à vous abonner.
- **TVmaze** : [politique de confidentialité](https://www.tvmaze.com/site/privacy).
  Données TV de [TVmaze](https://www.tvmaze.com/), sous licence
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
- **OMDb** : [conditions](https://www.omdbapi.com/legal.htm). Données de films
  de l'[API OMDb](https://www.omdbapi.com/), sous licence
  [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/). OMDb n'est
  ni approuvé par IMDb.com ni affilié à IMDb.com.

## Licence

GPL-3.0-or-later. Voir [LICENSE](../../../LICENSE).
