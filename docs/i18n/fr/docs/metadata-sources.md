<!-- eidos-i18n: source=docs/metadata-sources.md sha=8e251e1a15d6dffea5f3e2cc80a6d4a8eb0a93f1 -->

# Sources de métadonnées

KayaBot cherche les titres dans quatre bases de données en ligne. Cette page
indique à quoi sert chacune, comment KayaBot l'interroge et ce que le service
demande à un programme qui l'utilise. Ce qui quitte votre machine est résumé
dans la section [Confidentialité](../README.md#confidentialité) du README.

## En un coup d'œil

| Source | Utilisée pour | Clé | Confiance |
|---|---|---|---|
| TMDB | les films par défaut ; les séries si vous la choisissez | un jeton d'accès en lecture v4 ou une clé API v3 | 0.9 |
| TheTVDB | les séries par défaut | une clé API | 0.9 |
| TVmaze | les séries, en partenaire de TheTVDB | aucune | 0.75 |
| OMDb | les films, en partenaire de TMDB | une clé API | 0.7 |

La confiance est le poids fixe que chaque client donne à sa source quand les
résultats sont classés (voir
[architecture.md](architecture.md#dun-ancien-nom-à-un-nouveau)). Les libellés
affichés dans le programme proviennent de `MetadataSource::label` : TheMovieDB,
TheTVDB, TVmaze et OMDb.

## Comment KayaBot choisit les sources

- La liste Source de la carte Recherche, dans la vue Renommer, propose
  TheMovieDB, TheTVDB et TVmaze quand le type est Série, et TheMovieDB et OMDb
  quand c'est Film.
- Au démarrage, les séries utilisent TheTVDB et les films TheMovieDB. Votre
  dernier choix pour chaque type est conservé jusqu'à la fermeture du
  programme.
- Une recherche interroge ensemble la source choisie et ses partenaires :
  TheTVDB avec TVmaze, TVmaze avec TheTVDB, TMDB (pour les séries) avec les
  deux, et TMDB avec OMDb ou OMDb avec TMDB pour les films. Leurs réponses sont
  fusionnées : les partenaires ne sont donc pas un repli essayé après un échec,
  mais un second avis demandé en même temps.
- L'option « Utiliser cette source uniquement » n'interroge que la source
  choisie.
- Une source sans clé renvoie une erreur avant toute requête et est écartée de
  la fusion. TVmaze n'a pas besoin de clé : une recherche de série l'atteint
  donc toujours.

## Chaque source

### TMDB

- Hôte : `https://api.themoviedb.org/3`.
- Requêtes : `/search/multi` avec `query` et `include_adult=false` ;
  `/tv/{id}` puis `/tv/{id}/season/{n}` pour chaque saison afin de construire
  la liste des épisodes ; `/movie/{id}` pour un film.
- Authentification : une valeur qui commence par `eyJ` est un jeton d'accès en
  lecture v4 et part dans l'en-tête `Authorization: Bearer` ; toute autre
  valeur est une clé v3 et part dans le paramètre de requête `api_key`.
- Langue et région : `language` (par exemple `en-US`) sur chaque requête et
  `region` sur les recherches. Quand la langue et la région des métadonnées
  sont toutes deux vides dans les Préférences, KayaBot envoie `fr-FR`. Une
  langue seule reçoit sa région habituelle (`en` reçoit `US`, `fr` reçoit `FR`)
  et une région seule sa langue habituelle.
- Ce que KayaBot conserve : le titre localisé, avec le titre original comme
  alias quand il diffère ; l'année tirée de `release_date` (films) ou de
  `first_air_date` (séries) ; et `vote_average` divisé par 10 comme note de la
  source.
- Quota : plus de quota fixe depuis décembre 2019, mais TMDB répond encore par
  un `429` aux alentours de 40 requêtes par seconde
  ([limitation de débit](https://developer.themoviedb.org/docs/rate-limiting)).
- Conditions : les [conditions d'utilisation de l'API](https://www.themoviedb.org/api-terms-of-use)
  exigent la mention « This product uses the TMDB API but is not endorsed or
  certified by TMDB » et le respect des
  [règles de logo et d'attribution](https://www.themoviedb.org/about/logos-attribution).

### TheTVDB

- Hôte : `https://api4.thetvdb.com/v4`.
- Requêtes : `/search` avec `query` et `type=series` ;
  `/series/{id}/episodes/default` pour une liste d'épisodes ; `/movies/{id}`
  pour un film.
- Authentification : la clé est envoyée à `/login`, qui renvoie un jeton
  transmis en `Authorization: Bearer` sur les autres requêtes. KayaBot le
  renouvelle via `/refresh_token` et le garde en mémoire uniquement.
- Langue et région : la langue en paramètre de requête `language` et, avec la
  région, en `Accept-Language`.
- Ce que KayaBot conserve : le nom de la série, son `year`, et le `score` du
  résultat de recherche comme note de la source.
- Conditions : TheTVDB est gratuit pour les projets qui gagnent moins de
  50 000 $ par an, en échange d'une attribution avec un lien direct vers
  TheTVDB.com, visible par les personnes qui voient ses données
  ([informations sur l'API](https://thetvdb.com/api-information)). La
  formulation suggérée est « Metadata provided by TheTVDB. Please consider
  adding missing information or subscribing. »

### TVmaze

- Hôte : `https://api.tvmaze.com`.
- Requêtes : `/search/shows` avec `q` ; `/shows/{id}/episodes` pour une liste
  d'épisodes.
- Authentification : aucune. TVmaze demande aux clients d'envoyer un
  User-Agent qui les identifie ; KayaBot envoie `KayaBot`, sauf si
  `tvmaze_user_agent` ou `KAYABOT_TVMAZE_USER_AGENT` en définit un autre.
- Langue et région : aucune.
- Ce que KayaBot conserve : le nom de la série, l'année tirée de `premiered`,
  et le `score` du résultat de recherche (le score propre de la série quand il
  vaut 0) comme note de la source.
- Quota : au moins 20 appels toutes les 10 secondes par adresse IP.
- Conditions : l'[API](https://www.tvmaze.com/api) est sous licence
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) : TVmaze doit
  être cité comme source, et les données partagées suivent les conditions
  ShareAlike (partage dans les mêmes conditions).

### OMDb

- Hôte : `https://www.omdbapi.com`.
- Requêtes : `?s=<titre>` pour chercher et `?i=<identifiant IMDb>` pour les
  détails d'un film.
- Authentification : la clé part dans le paramètre de requête `apikey`.
- Langue et région : aucune.
- Ce que KayaBot conserve : le titre, les quatre premiers chiffres de `Year`,
  et l'identifiant IMDb, qui sert aussi d'identifiant au résultat. OMDb ne
  fournit aucune note que KayaBot utilise : chaque résultat reçoit une note de
  source fixe de 0.7.
- Quota : une clé gratuite permet 1 000 requêtes par jour
  ([clés API](https://www.omdbapi.com/apikey.aspx)).
- Conditions : le contenu d'OMDb est sous licence
  [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/), et OMDb
  n'est ni approuvé par IMDb.com ni affilié à IMDb.com
  ([conditions](https://www.omdbapi.com/legal.htm)).

## Limites et erreurs

Chaque requête a un délai d'expiration de 10 secondes. Une réponse
`429 Too Many Requests` est signalée comme une limite de débit, et une source
qui expire ou échoue est écartée de cette recherche sans arrêter les autres.
Les messages d'erreur n'incluent jamais l'URL de la requête : une clé envoyée
dans la chaîne de requête n'atteint donc pas la fenêtre.

Les quotas et les conditions changent : consultez les pages de chaque service
avant de vous fier à un chiffre.
