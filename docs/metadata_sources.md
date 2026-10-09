# Sources de métadonnées

## Tableau comparatif (par source)

> Les colonnes sont basées sur l'énumération `MetadataSource` et ses labels.
> Les champs "fiabilité/confiance" et "coût d'intégration" sont à compléter après tests d'API.

| Critère | TheMovieDB | TheTVDB | TVmaze | OMDb |
| --- | --- | --- | --- | --- |
| Couverture (séries/films/anime) | Films + séries | Séries | Séries | Films |
| Stabilité des IDs | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) | IDs propriétaires (stabilité à valider) |
| Richesse (cast, images, épisodes, saisons) | Cast + images + saisons/épisodes | Cast + images + saisons/épisodes | Cast + images + saisons/épisodes (cast/images à confirmer) | Cast + images (épisodes/saisons non applicables) |
| Fiabilité / confiance | À tester | À tester | À tester | À tester |
| Coût d'intégration | À tester | À tester | À tester | À tester |

## Grille “API access” (auth, quotas, redistribution)

| Source | Mode d'authentification | Quotas gratuits / tiers payants | Contraintes de redistribution / usage |
| --- | --- | --- | --- |
| **TheMovieDB (TMDB)** | Clé API v3 (API key) ou token Bearer v4. OAuth disponible pour actions utilisateur. | Limites de taux par IP (ex. ~40 requêtes / 10s). Plans pro/enterprise pour usages à fort volume. | Attribution obligatoire (logo TMDB + mention). Redistribution de données limitée par les conditions TMDB. |
| **TheTVDB** | API v4 : clé API + token JWT via login. | Accès soumis à abonnement, quotas selon plan (gratuits très limités ou absents). | Attribution/branding TheTVDB requis. Restrictions pour usage commercial et redistribution (selon plan). |
| **TVmaze** | Pas d'auth obligatoire pour les endpoints publics (clé optionnelle pour certains usages). | Rate limit public (ex. ~20 requêtes / 10s). Pas de plan payant officiel, mais contact requis pour usage lourd. | Attribution conseillée, redistribution limitée aux conditions d'utilisation TVmaze. |
| **OMDb** | Clé API obligatoire. | Gratuit limité (ex. ~1 000 requêtes / jour). Abonnement payant pour volume plus élevé. | Usage commercial soumis à licence payante; attribution recommandée. |

> Notes : les quotas/conditions peuvent évoluer. Vérifier les pages officielles avant intégration.

## Comportements actuels côté app

### Sources disponibles et labels

Les labels affichés dans l'UI proviennent de `MetadataSource::label` :

- TheMovieDB, TheTVDB, TVmaze, OMDb.

### Sélection par type de contenu

- **Mode séries (Episode Mode)** : choix possibles = TheMovieDB, TheTVDB, TVmaze.
- **Mode films (Movie Mode)** : choix possibles = TheMovieDB, OMDb.

### Sources préférées par défaut

- **Séries** : TheTVDB (source active et préférée au démarrage).
- **Films** : TheMovieDB.

Ces valeurs sont initialisées dans l'état de l'app (`RenameApp::default`) et mises à jour lors du changement de source.

### Fallbacks et forçage

- Par défaut, l'app active la source choisie **plus** ses fallbacks :
  - Films : TMDB ↔ OMDb.
  - Séries : TheTVDB ↔ TVmaze.
- L'option "Forcer la source" limite les requêtes à la source active uniquement.
