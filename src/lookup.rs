//! Metadata lookups for a batch of files: which sources to ask, what to ask
//! them, and how to read the answers. The functions taking a pipeline block on
//! the network, so the interface runs them off its own thread.

use crate::api_keys::ApiConfig;
use crate::matching::{ContentType, MatchMetadata, MatchResult};
use crate::metadata::aggregate::MetadataPipeline;
use crate::metadata::error::MetadataError;
use crate::metadata::locale::MetadataLocale;
use crate::metadata::models::{EpisodeMatch, TitleMatch};
use crate::metadata::provider::{MetadataProvider, MetadataSource};
use crate::metadata::providers::{
    omdb::OmdbClient, thetvdb::TheTvDbClient, tmdb::TmdbClient, tvmaze::TvMazeClient,
};

/// The four sources, with the user's keys and locale. Cheap: the HTTP clients
/// are only built when a request is made.
pub fn pipeline(api: &ApiConfig, locale: Option<MetadataLocale>) -> MetadataPipeline {
    MetadataPipeline::new(vec![
        (
            MetadataSource::TheMovieDb,
            Box::new(TmdbClient::new(api.tmdb_token.clone(), locale.clone())),
        ),
        (
            MetadataSource::TheTvDb,
            Box::new(TheTvDbClient::new(api.tvdb_api_key.clone(), locale)),
        ),
        (
            MetadataSource::TvMaze,
            Box::new(TvMazeClient::new(api.tvmaze_user_agent.clone())),
        ),
        (
            MetadataSource::Omdb,
            Box::new(OmdbClient::new(api.omdb_api_key.clone())),
        ),
    ])
}

/// Which sources a lookup asks, and in what order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePlan {
    /// Ask `chosen` and nothing else.
    pub force: bool,
    /// The source the user picked last.
    pub chosen: MetadataSource,
    pub preferred_movie: MetadataSource,
    pub preferred_series: MetadataSource,
}

impl Default for SourcePlan {
    fn default() -> Self {
        Self {
            force: false,
            chosen: MetadataSource::TheTvDb,
            preferred_movie: MetadataSource::TheMovieDb,
            preferred_series: MetadataSource::TheTvDb,
        }
    }
}

impl SourcePlan {
    /// The source asked first for this kind of content.
    pub fn primary(&self, content_type: ContentType) -> MetadataSource {
        if self.force {
            self.chosen
        } else {
            self.preferred(content_type)
        }
    }

    pub fn preferred(&self, content_type: ContentType) -> MetadataSource {
        match content_type {
            ContentType::Movie => self.preferred_movie,
            ContentType::Series => self.preferred_series,
        }
    }

    /// The primary source, then its fallbacks unless the source is forced.
    pub fn sources(&self, content_type: ContentType) -> Vec<MetadataSource> {
        let primary = self.primary(content_type);
        let mut sources = vec![primary];
        if !self.force {
            sources.extend(fallback_sources(content_type, primary));
        }
        sources
    }
}

/// The sources that can answer for this kind of content.
pub fn sources_for(content_type: ContentType) -> &'static [MetadataSource] {
    match content_type {
        ContentType::Movie => &[MetadataSource::TheMovieDb, MetadataSource::Omdb],
        ContentType::Series => &[
            MetadataSource::TheMovieDb,
            MetadataSource::TheTvDb,
            MetadataSource::TvMaze,
        ],
    }
}

fn fallback_sources(content_type: ContentType, primary: MetadataSource) -> Vec<MetadataSource> {
    match content_type {
        ContentType::Movie => match primary {
            MetadataSource::TheMovieDb => vec![MetadataSource::Omdb],
            MetadataSource::Omdb => vec![MetadataSource::TheMovieDb],
            _ => vec![MetadataSource::TheMovieDb, MetadataSource::Omdb],
        },
        ContentType::Series => match primary {
            MetadataSource::TheTvDb => vec![MetadataSource::TvMaze],
            MetadataSource::TvMaze => vec![MetadataSource::TheTvDb],
            _ => vec![MetadataSource::TheTvDb, MetadataSource::TvMaze],
        },
    }
}

/// The kind of content a result is, from its detection or its parsed name.
pub fn content_type_of(result: &MatchResult) -> Option<ContentType> {
    result.content_type.or(match result.metadata {
        Some(MatchMetadata::Movie { .. }) => Some(ContentType::Movie),
        Some(MatchMetadata::Series { .. }) | Some(MatchMetadata::Episode { .. }) => {
            Some(ContentType::Series)
        }
        None => None,
    })
}

/// The season and episode parsed from the file name.
pub fn episode_hint(result: &MatchResult) -> Option<(u32, u32)> {
    match &result.metadata {
        Some(MatchMetadata::Episode {
            season, episode, ..
        }) => Some((*season, *episode)),
        _ => None,
    }
}

/// What to search for one file: its parsed title, or `detected_name` when the
/// name had none, plus the year for a movie.
pub fn query_for(result: &MatchResult, detected_name: &str) -> Option<String> {
    let detected = detected_name.trim();
    let detected = || (!detected.is_empty()).then(|| detected.to_string());
    match &result.metadata {
        Some(MatchMetadata::Series { title }) => title.clone().or_else(detected),
        Some(MatchMetadata::Episode { series_title, .. }) => series_title.clone().or_else(detected),
        Some(MatchMetadata::Movie { title, year }) => {
            let title = title.clone()?;
            Some(match year {
                Some(year) => format!("{title} {year}"),
                None => title,
            })
        }
        None => None,
    }
}

/// The candidate with the highest global score.
pub fn best_title(titles: &[TitleMatch]) -> Option<&TitleMatch> {
    titles.iter().max_by(|a, b| {
        a.global_score
            .partial_cmp(&b.global_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Looks every result up: the best title for its query, then for an episode
/// the best entry of that title's episode list. A result that finds nothing
/// keeps what its file name said.
pub fn match_results(
    pipeline: &mut MetadataPipeline,
    mut results: Vec<MatchResult>,
    plan: &SourcePlan,
    detected_name: &str,
) -> Vec<MatchResult> {
    for result in &mut results {
        let content_type = content_type_of(result).unwrap_or(ContentType::Series);
        pipeline.set_active_sources(plan.sources(content_type));
        match_one(pipeline, result, detected_name);
    }
    results
}

fn match_one(pipeline: &mut MetadataPipeline, result: &mut MatchResult, detected_name: &str) {
    let Some(query) = query_for(result, detected_name) else {
        return;
    };
    let Ok(titles) = pipeline.search_title(&query) else {
        return;
    };
    result.metadata_candidate_count = Some(titles.len());
    let Some(best) = best_title(&titles) else {
        return;
    };
    result.metadata_confidence = Some(best.global_score);
    result.title_match = Some(best.clone());

    let Some((season, episode)) = episode_hint(result) else {
        return;
    };
    let Ok(episodes) = pipeline.fetch_episode_list(&best.id) else {
        return;
    };
    let candidates: Vec<&EpisodeMatch> = episodes
        .iter()
        .filter(|candidate| candidate.season == season && candidate.episode == episode)
        .collect();
    if !candidates.is_empty() {
        result.metadata_candidate_count = Some(candidates.len());
    }
    // The first of equally scored episodes wins.
    let best_episode = candidates.into_iter().reduce(|best, candidate| {
        if candidate.global_score > best.global_score {
            candidate
        } else {
            best
        }
    });
    if let Some(best_episode) = best_episode {
        result.metadata_confidence = Some(best_episode.global_score);
        result.episode_match = Some(best_episode.clone());
    }
}

/// What a search for one title found.
#[derive(Debug, Clone)]
pub struct FetchOutcome {
    pub titles: Result<Vec<TitleMatch>, MetadataError>,
    /// The episode list of the best title, when episodes were asked for.
    pub episodes: Option<Result<Vec<EpisodeMatch>, MetadataError>>,
    /// The source that answered.
    pub source: MetadataSource,
}

/// Searches `query` on `sources`, then, for a series, loads the episode list
/// of the best title.
pub fn fetch(
    pipeline: &mut MetadataPipeline,
    query: &str,
    sources: Vec<MetadataSource>,
    with_episodes: bool,
) -> FetchOutcome {
    pipeline.set_active_sources(sources);
    let titles = pipeline.search_title(query);
    let episodes = match &titles {
        Ok(titles) if with_episodes => {
            best_title(titles).map(|best| pipeline.fetch_episode_list(&best.id))
        }
        _ => None,
    };
    FetchOutcome {
        titles,
        episodes,
        source: pipeline.active_source(),
    }
}

/// The episode list of one title, and the source that answered.
pub fn episodes(
    pipeline: &mut MetadataPipeline,
    title_id: &str,
) -> (Result<Vec<EpisodeMatch>, MetadataError>, MetadataSource) {
    let episodes = pipeline.fetch_episode_list(title_id);
    (episodes, pipeline.active_source())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::matching;
    use crate::metadata::models::{MetadataExtras, MovieMatch};

    /// A source that knows one series with two episodes.
    pub struct MockProvider {
        title: TitleMatch,
        episodes: Vec<EpisodeMatch>,
    }

    impl MockProvider {
        pub fn new(name: &str) -> Self {
            let episode = |number: u32, title: &str| EpisodeMatch {
                id: format!("e{number}"),
                season: 1,
                episode: number,
                title: title.to_string(),
                source_score: 0.9,
                source_trust: 0.9,
                global_score: 0.0,
                source: MetadataSource::TheMovieDb.label().to_string(),
                extras: MetadataExtras::default(),
            };
            Self {
                title: TitleMatch {
                    id: "mock-series".to_string(),
                    name: name.to_string(),
                    year: Some(2022),
                    source_score: 0.95,
                    source_trust: 0.9,
                    global_score: 0.0,
                    source: MetadataSource::TheMovieDb.label().to_string(),
                    extras: MetadataExtras::default(),
                },
                episodes: vec![episode(1, "Pilot"), episode(5, "Chapter Five")],
            }
        }

        pub fn pipeline(name: &str) -> MetadataPipeline {
            MetadataPipeline::new(vec![(
                MetadataSource::TheMovieDb,
                Box::new(Self::new(name)),
            )])
        }
    }

    impl MetadataProvider for MockProvider {
        fn search_title(&mut self, _query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
            Ok(vec![self.title.clone()])
        }

        fn fetch_episode_list(&mut self, _id: &str) -> Result<Vec<EpisodeMatch>, MetadataError> {
            Ok(self.episodes.clone())
        }

        fn fetch_movie_details(&mut self, _id: &str) -> Result<MovieMatch, MetadataError> {
            Err(MetadataError::NotFound("no movies".to_string()))
        }
    }

    #[test]
    fn a_forced_source_has_no_fallback() {
        let mut plan = SourcePlan::default();
        assert_eq!(
            plan.sources(ContentType::Series),
            [MetadataSource::TheTvDb, MetadataSource::TvMaze]
        );
        assert_eq!(
            plan.sources(ContentType::Movie),
            [MetadataSource::TheMovieDb, MetadataSource::Omdb]
        );
        plan.force = true;
        plan.chosen = MetadataSource::TvMaze;
        assert_eq!(plan.sources(ContentType::Movie), [MetadataSource::TvMaze]);
    }

    #[test]
    fn a_movie_query_carries_its_year() {
        let results = matching::match_files(&[
            "Spirited.Away.2001.avi".to_string(),
            "s01e02.mkv".to_string(),
        ]);
        assert_eq!(
            query_for(&results[0], "").as_deref(),
            Some("Spirited Away 2001")
        );
        assert_eq!(query_for(&results[1], " Lupin ").as_deref(), Some("Lupin"));
    }

    #[test]
    fn fetch_returns_titles_and_the_best_titles_episodes() {
        let mut pipeline = MockProvider::pipeline("Lupin");
        let outcome = fetch(
            &mut pipeline,
            "Lupin",
            vec![MetadataSource::TheMovieDb],
            true,
        );
        assert_eq!(outcome.titles.expect("titles").len(), 1);
        assert_eq!(outcome.episodes.expect("asked").expect("episodes").len(), 2);
        assert_eq!(outcome.source, MetadataSource::TheMovieDb);

        let outcome = fetch(
            &mut pipeline,
            "Lupin",
            vec![MetadataSource::TheMovieDb],
            false,
        );
        assert!(outcome.episodes.is_none());
    }

    #[test]
    fn matching_fills_the_title_and_the_episode() {
        let mut pipeline = MockProvider::pipeline("Lupin");
        let plan = SourcePlan {
            preferred_series: MetadataSource::TheMovieDb,
            ..SourcePlan::default()
        };
        let results = matching::match_files(&["Lupin.s01e05.mkv".to_string()]);

        let results = match_results(&mut pipeline, results, &plan, "");

        let result = &results[0];
        assert_eq!(
            result.title_match.as_ref().map(|t| t.name.as_str()),
            Some("Lupin")
        );
        let episode = result.episode_match.as_ref().expect("episode matched");
        assert_eq!((episode.season, episode.episode), (1, 5));
        assert_eq!(result.metadata_candidate_count, Some(1));
    }
}
