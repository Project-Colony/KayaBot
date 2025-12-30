# Sources de métadonnées

## Tableau comparatif (par source)

> Les colonnes sont basées sur l'énumération `MetadataSource` et ses labels.
> Les champs "fiabilité/confiance" et "coût d'intégration" sont à compléter après tests d'API.

| Critère | TheMovieDB | AniDB | TheTVDB | TVmaze | OMDb |
| --- | --- | --- | --- | --- | --- |
| Couverture (séries/films/anime) | Films + séries | Anime (séries/films) | Séries | Séries | Films |
| Stabilité des IDs | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) |
| Richesse (cast, images, épisodes, saisons) | Cast + images + saisons/épisodes | Épisodes/saisons + infos anime (cast/images à confirmer) | Cast + images + saisons/épisodes | Cast + images + saisons/épisodes (cast/images à confirmer) | Cast + images (épisodes/saisons non applicables) |
| Fiabilité / confiance | À tester | À tester | À tester | À tester | À tester |
| Coût d'intégration | À tester | À tester | À tester | À tester | À tester |

## Comportements actuels côté app

### Sources disponibles et labels

Les labels affichés dans l'UI proviennent de `MetadataSource::label` :

- TheMovieDB, AniDB, TheTVDB, TVmaze, OMDb.

### Sélection par type de contenu

- **Mode séries (Episode Mode)** : choix possibles = TheMovieDB, AniDB, TheTVDB, TVmaze.
- **Mode films (Movie Mode)** : choix possibles = TheMovieDB, OMDb.

### Sources préférées par défaut

- **Séries** : TheTVDB (source active et préférée au démarrage).
- **Films** : TheMovieDB.

Ces valeurs sont initialisées dans l'état de l'app (`RenameApp::default`) et mises à jour lors du changement de source.
