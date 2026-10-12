# Metadata sources

KayaBot looks titles up in four online databases. This page says what each one
is used for, how KayaBot talks to it, and what the service asks of a program
that uses it. What leaves your machine is summed up in the README's
[Privacy](../README.md#privacy) section.

## At a glance

| Source | Used for | Key | Trust |
|---|---|---|---|
| TMDB | movies by default; series when you pick it | a v4 read access token or a v3 API key | 0.9 |
| TheTVDB | series by default | an API key | 0.9 |
| TVmaze | series, as TheTVDB's partner | none | 0.75 |
| OMDb | movies, as TMDB's partner | an API key | 0.7 |

Trust is the fixed weight each client gives its source when results are ranked
(see [architecture.md](architecture.md#from-an-old-name-to-a-new-one)). The
labels shown in the program come from `MetadataSource::label`: TheMovieDB,
TheTVDB, TVmaze and OMDb.

## How KayaBot picks sources

- The Source list of the Lookup card, in the Rename view, offers TheMovieDB,
  TheTVDB and TVmaze when the type is Series, and TheMovieDB and OMDb when it is
  Movie.
- At start, series use TheTVDB and movies use TheMovieDB. Your last pick for
  each type is kept until you close the program.
- A search asks the chosen source and its partners together: TheTVDB with
  TVmaze, TVmaze with TheTVDB, TMDB (for series) with both, and TMDB with OMDb
  or OMDb with TMDB for movies. Their answers are merged, so the partners are
  not a fallback tried after a failure but a second opinion asked at the same
  time.
- The "Use this source only" option asks the chosen source only.
- A source without a key returns an error before any request and is left out of
  the merge. TVmaze needs no key, so a series search always reaches it.

## Each source

### TMDB

- Host: `https://api.themoviedb.org/3`.
- Requests: `/search/multi` with `query` and `include_adult=false`;
  `/tv/{id}` and then `/tv/{id}/season/{n}` for every season to build an
  episode list; `/movie/{id}` for a movie.
- Authentication: a value starting with `eyJ` is a v4 read access token and
  goes in the `Authorization: Bearer` header; anything else is a v3 key and goes
  in the `api_key` query parameter.
- Locale: `language` (for example `en-US`) on every request and `region` on
  searches. When the metadata language and region in Preferences are both
  unset, KayaBot sends `fr-FR`. A language alone gets its usual region (`en`
  gets `US`, `fr` gets `FR`) and a region alone its usual language.
- Terms: the [API terms of use](https://www.themoviedb.org/api-terms-of-use)
  require the notice "This product uses the TMDB API but is not endorsed or
  certified by TMDB" and follow the
  [logo and attribution rules](https://www.themoviedb.org/about/logos-attribution).

### TheTVDB

- Host: `https://api4.thetvdb.com/v4`.
- Requests: `/search` with `query` and `type=series`;
  `/series/{id}/episodes/default` for an episode list; `/movies/{id}` for a
  movie.
- Authentication: the key is posted to `/login`, which returns a token sent as
  `Authorization: Bearer` on the other requests. KayaBot renews it through
  `/refresh_token` and keeps it in memory only.
- Locale: the language as a `language` query parameter and, with the region, as
  `Accept-Language`.
- Terms: TheTVDB is free for projects earning less than $50k a year, against
  attribution with a direct link to TheTVDB.com, shown to the people who see
  its data ([API information](https://thetvdb.com/api-information)). The
  suggested wording is "Metadata provided by TheTVDB. Please consider adding
  missing information or subscribing."

### TVmaze

- Host: `https://api.tvmaze.com`.
- Requests: `/search/shows` with `q`; `/shows/{id}/episodes` for an episode
  list.
- Authentication: none. TVmaze asks clients to send a User-Agent that
  identifies them; KayaBot sends `KayaBot` unless `tvmaze_user_agent` or
  `KAYABOT_TVMAZE_USER_AGENT` sets another one.
- Locale: none.
- Terms: the [API](https://www.tvmaze.com/api) is licensed under
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/): TVmaze must
  be credited as the source, and shared data follows the ShareAlike terms.

### OMDb

- Host: `https://www.omdbapi.com`.
- Requests: `?s=<title>` to search and `?i=<IMDb ID>` for a movie's details.
- Authentication: the key goes in the `apikey` query parameter.
- Locale: none.
- Terms: OMDb content is licensed under
  [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/), and OMDb is
  not endorsed by or affiliated with IMDb.com
  ([terms](https://www.omdbapi.com/legal.htm)).

## Limits and errors

Every request has a 10 second timeout. A `429 Too Many Requests` answer is
reported as a rate limit, and a source that times out or fails is left out of
that search without stopping the others. Error messages never include the
request URL, so a key sent in the query string does not reach the window.

Quotas and terms change: check each service's own pages before relying on a
number.
