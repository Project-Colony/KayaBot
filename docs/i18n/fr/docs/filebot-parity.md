<!-- eidos-i18n: source=docs/filebot-parity.md sha=40014280219be7504849fc6b64aef7592b0a9b2f -->

# Parité avec FileBot

FileBot est la référence de ce qu'un renommeur de médias doit savoir faire.
Cette page liste les fonctionnalités de FileBot que KayaBot vise à couvrir et
où chacune en est aujourd'hui, pour que personne n'ait à lire le code pour le
savoir.

**Oui** signifie que cela fonctionne dans le programme actuel. **En partie**
signifie qu'une version réduite existe. **Pas encore** signifie qu'aucun code
n'existe pour cela.

## Renommage et organisation

| Fonctionnalité | KayaBot aujourd'hui |
|---|---|
| Renommer selon un modèle | **Oui.** Modèles prédéfinis et personnalisés pour les épisodes et les films |
| Conserver l'extension | **Oui** |
| Gérer les collisions de noms | **Oui.** N'écrase jamais rien ; un nom déjà pris reçoit ` (1)`, ` (2)` et ainsi de suite, et deux fichiers d'un même lot n'entrent jamais en collision |
| Prévisualisation et simulation | **Oui.** Chaque nouveau nom est affiché d'abord, et une simulation (dry run) ne renomme rien |
| Déplacer les fichiers dans des dossiers par série, saison, film ou collection | **Pas encore.** Les fichiers sont renommés dans leur propre dossier |
| Renommer les sous-titres avec leur vidéo | **Pas encore** |
| Normaliser les suffixes de langue des sous-titres (`.fr`, `.en`, `.multi`) | **Pas encore** |
| Télécharger des sous-titres | **Pas encore** |
| Écrire des fichiers `.nfo` ou des tags dans les fichiers | **Pas encore** |
| Vérifier les SFV ou d'autres sommes de contrôle | **Pas encore** |

## Correspondance

| Fonctionnalité | KayaBot aujourd'hui |
|---|---|
| Lire la saison, l'épisode et l'année dans un nom de fichier | **Oui.** `S01E02`, `S01 E02`, `1x02`, `102`, `Season 1 Episode 2`, une année de 1900 à 2099 |
| Distinguer un épisode d'un film | **Oui**, d'après le nom analysé ; vous pouvez changer le type |
| Identifier les séries et les épisodes | **Oui.** Recherche par titre, puis l'épisode par saison et numéro |
| Identifier les films | **Oui.** Titre et année |
| Choisir entre des résultats ambigus | **En partie.** Les candidats sont listés avec leur score et vous pouvez en choisir un autre, ou corriger à la main le titre, la saison et l'épisode d'un fichier. Il n'y a pas de question interactive pendant un lot |
| Identifier par empreinte (hash) du fichier | **Pas encore** |
| Éditions et versions (cuts), fichiers multi-épisodes, épisodes spéciaux | **Pas encore** |
| Filtrer les fichiers par règles | **Pas encore** |

## Métadonnées

| Fonctionnalité | KayaBot aujourd'hui |
|---|---|
| Plusieurs sources avec une priorité par type de contenu | **Oui.** TheTVDB et TVmaze pour les séries, TMDB et OMDb pour les films ; voir [metadata-sources.md](metadata-sources.md) |
| Fusionner les résultats de plusieurs sources | **Oui.** Par identifiants communs et similarité des titres |
| Se rabattre sur une autre source quand l'une échoue | **Oui.** Les partenaires sont interrogés en même temps et une source en échec est écartée |
| Titre officiel, année et titre d'épisode | **Oui** |
| Synopsis et genres | **En partie.** Affichés à côté d'une correspondance, pas utilisés dans les noms |
| Langue et région des titres | **Oui** pour TMDB et TheTVDB |
| Distribution, illustrations, collections | **Pas encore** |

## Modèles

KayaBot lit des jetons au format FileBot, un petit sous-ensemble du langage de
format de FileBot :

| Jeton | KayaBot aujourd'hui |
|---|---|
| `{n}` titre, `{t}` titre d'épisode, `{y}` année | **Oui** |
| `{s}`, `{e}` et leurs formes complétées par des zéros comme `{s00}`, `{e00}` | **Oui** |
| Supprimer une valeur manquante avec son séparateur | **Oui** |
| `{airdate}` et les autres variables de FileBot | **Pas encore** |
| Expressions et conditions dans un modèle | **Pas encore** |
