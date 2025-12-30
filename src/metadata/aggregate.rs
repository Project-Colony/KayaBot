use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

use crate::matching::rank_candidates;
use crate::metadata::error::MetadataError;
use crate::metadata::models::{ExternalId, ExternalIds, TitleMatch};
use crate::metadata::provider::{MetadataProvider, MetadataSource};

const GLOBAL_TRUST_WEIGHT: f32 = 0.55;
const GLOBAL_FUZZY_WEIGHT: f32 = 0.35;
const GLOBAL_SOURCE_WEIGHT: f32 = 0.10;
const TITLE_SIMILARITY_THRESHOLD: f32 = 0.9;

pub struct MetadataPipeline {
    providers: Vec<ProviderEntry>,
    active_sources: Vec<MetadataSource>,
    active_source: MetadataSource,
}

struct ProviderEntry {
    source: MetadataSource,
    provider: Box<dyn MetadataProvider>,
}

struct TitleGroup {
    best: TitleMatch,
    insertion_index: usize,
}

impl MetadataPipeline {
    pub fn new(entries: Vec<(MetadataSource, Box<dyn MetadataProvider>)>) -> Self {
        let providers: Vec<ProviderEntry> = entries
            .into_iter()
            .map(|(source, provider)| ProviderEntry { source, provider })
            .collect();
        let active_sources = providers.iter().map(|entry| entry.source).collect();
        let active_source = providers
            .first()
            .map(|entry| entry.source)
            .unwrap_or(MetadataSource::TheMovieDb);
        Self {
            providers,
            active_sources,
            active_source,
        }
    }

    pub fn set_sources(&mut self, primary: MetadataSource, secondary: Option<MetadataSource>) {
        let mut sources = vec![primary];
        if let Some(secondary) = secondary {
            if secondary != primary {
                sources.push(secondary);
            }
        }
        self.set_active_sources(sources);
    }

    pub fn set_active_sources(&mut self, sources: Vec<MetadataSource>) {
        let mut unique = Vec::new();
        for source in sources {
            if self.providers.iter().any(|entry| entry.source == source)
                && !unique.contains(&source)
            {
                unique.push(source);
            }
        }
        if unique.is_empty() {
            return;
        }
        self.active_sources = unique;
        if !self.active_sources.contains(&self.active_source) {
            self.active_source = self.active_sources[0];
        }
    }

    pub fn active_source(&self) -> MetadataSource {
        self.active_source
    }

    fn provider_mut(&mut self, source: MetadataSource) -> Option<&mut (dyn MetadataProvider + '_)> {
        let entry = self
            .providers
            .iter_mut()
            .find(|entry| entry.source == source)?;
        Some(entry.provider.as_mut())
    }
}

impl MetadataProvider for MetadataPipeline {
    fn search_title(&mut self, query: &str) -> Result<Vec<TitleMatch>, MetadataError> {
        if self.active_sources.is_empty() {
            return Err(MetadataError::Other(
                "No metadata providers are configured.".to_string(),
            ));
        }

        let query = query.trim();
        if query.is_empty() {
            return Err(MetadataError::InvalidResponse(
                "Search query cannot be empty.".to_string(),
            ));
        }

        let query_owned = query.to_string();
        let active_sources = self.active_sources.clone();
        let source_rank: HashMap<_, _> = active_sources
            .iter()
            .enumerate()
            .map(|(idx, source)| (*source, idx))
            .collect();
        let (tx, rx) = mpsc::channel();

        std::thread::scope(|scope| {
            let active: HashSet<_> = self.active_sources.iter().copied().collect();
            for entry in self
                .providers
                .iter_mut()
                .filter(|entry| active.contains(&entry.source))
            {
                let source = entry.source;
                let provider = entry.provider.as_mut();
                let tx = tx.clone();
                let query = query_owned.clone();
                scope.spawn(move || {
                    let result = provider.search_title(&query);
                    let _ = tx.send((source, result));
                });
            }
        });
        drop(tx);

        let mut collected = Vec::new();
        for (source, result) in rx {
            collected.push((source, result));
        }

        let mut errors = Vec::new();
        let mut ordered = Vec::new();
        collected.sort_by_key(|(source, _)| source_rank.get(source).copied().unwrap_or(usize::MAX));

        for (source, result) in collected {
            match result {
                Ok(matches) => {
                    ordered.push((source, matches));
                }
                Err(err) => errors.push(err),
            }
        }

        if ordered.is_empty() {
            return Err(errors.pop().unwrap_or_else(|| {
                MetadataError::Other("No metadata providers responded.".into())
            }));
        }

        let mut candidates = Vec::new();
        let mut insertion_index = 0usize;
        for (source, mut matches) in ordered {
            for mut title in matches.drain(..) {
                let fuzzy_score = fuzzy_score(query, &title.name);
                let global_score = (title.source_trust * GLOBAL_TRUST_WEIGHT)
                    + (fuzzy_score * GLOBAL_FUZZY_WEIGHT)
                    + (title.source_score * GLOBAL_SOURCE_WEIGHT);
                title.global_score = global_score;
                title.id = format!("{}:{}", source.label(), title.id);
                title.source = source.label().to_string();
                candidates.push((insertion_index, title));
                insertion_index += 1;
            }
        }

        let mut id_to_group: HashMap<String, usize> = HashMap::new();
        let mut groups: Vec<TitleGroup> = Vec::new();

        for (index, title) in candidates {
            let mut group_index = None;

            for key in external_id_keys(&title.extras.external_ids) {
                if let Some(existing) = id_to_group.get(&key) {
                    group_index = Some(*existing);
                    break;
                }
            }

            if group_index.is_none() {
                for (idx, group) in groups.iter().enumerate() {
                    if !year_compatible(&group.best, &title) {
                        continue;
                    }
                    let similarity = fuzzy_score(&group.best.name, &title.name);
                    if similarity >= TITLE_SIMILARITY_THRESHOLD {
                        group_index = Some(idx);
                        break;
                    }
                }
            }

            let resolved_index = match group_index {
                Some(idx) => {
                    merge_group(&mut groups[idx], title, index);
                    idx
                }
                None => {
                    groups.push(TitleGroup {
                        best: title,
                        insertion_index: index,
                    });
                    groups.len() - 1
                }
            };

            for key in external_id_keys(&groups[resolved_index].best.extras.external_ids) {
                id_to_group.insert(key, resolved_index);
            }
        }

        let mut results: Vec<(usize, TitleMatch)> = groups
            .into_iter()
            .map(|group| (group.insertion_index, group.best))
            .collect();

        results.sort_by(|(a_idx, a), (b_idx, b)| {
            b.global_score
                .partial_cmp(&a.global_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a_idx.cmp(b_idx))
        });

        if let Some(best) = results.first().map(|(_, title)| title) {
            if let Some(source) = MetadataSource::from_label(&best.source) {
                self.active_source = source;
            }
        }

        Ok(results.into_iter().map(|(_, title)| title).collect())
    }

    fn fetch_episode_list(
        &mut self,
        title_id: &str,
    ) -> Result<Vec<crate::metadata::models::EpisodeMatch>, MetadataError> {
        let (source, id) = split_source_id(title_id);
        if let Some(source) = source {
            let provider = self
                .provider_mut(source)
                .ok_or_else(|| MetadataError::Other("Unknown metadata source.".to_string()))?;
            let result = provider.fetch_episode_list(&id);
            if result.is_ok() {
                self.active_source = source;
            }
            return result;
        }

        for source in self.active_sources.clone() {
            if let Some(provider) = self.provider_mut(source) {
                if let Ok(result) = provider.fetch_episode_list(title_id) {
                    self.active_source = source;
                    return Ok(result);
                }
            }
        }

        Err(MetadataError::NotFound(format!(
            "No episode data found for '{title_id}'."
        )))
    }

    fn fetch_movie_details(
        &mut self,
        title_id: &str,
    ) -> Result<crate::metadata::models::MovieMatch, MetadataError> {
        let (source, id) = split_source_id(title_id);
        if let Some(source) = source {
            let provider = self
                .provider_mut(source)
                .ok_or_else(|| MetadataError::Other("Unknown metadata source.".to_string()))?;
            let result = provider.fetch_movie_details(&id);
            if result.is_ok() {
                self.active_source = source;
            }
            return result;
        }

        for source in self.active_sources.clone() {
            if let Some(provider) = self.provider_mut(source) {
                if let Ok(result) = provider.fetch_movie_details(title_id) {
                    self.active_source = source;
                    return Ok(result);
                }
            }
        }

        Err(MetadataError::NotFound(format!(
            "No movie data found for '{title_id}'."
        )))
    }
}

fn split_source_id(title_id: &str) -> (Option<MetadataSource>, String) {
    let Some((prefix, id)) = title_id.split_once(':') else {
        return (None, title_id.to_string());
    };
    let source = MetadataSource::from_label(prefix);
    (source, id.to_string())
}

fn fuzzy_score(query: &str, candidate: &str) -> f32 {
    let ranked = rank_candidates(query, &[candidate.to_string()]);
    ranked.first().map(|item| item.score).unwrap_or(0.0)
}

fn external_id_keys(ids: &ExternalIds) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(imdb) = &ids.imdb {
        keys.push(format!("imdb:{imdb}"));
    }
    if let Some(tmdb) = &ids.tmdb {
        keys.push(format!("tmdb:{tmdb}"));
    }
    if let Some(tvdb) = &ids.tvdb {
        keys.push(format!("tvdb:{tvdb}"));
    }
    if let Some(tvmaze) = &ids.tvmaze {
        keys.push(format!("tvmaze:{tvmaze}"));
    }
    if let Some(anidb) = &ids.anidb {
        keys.push(format!("anidb:{anidb}"));
    }
    if let Some(omdb) = &ids.omdb {
        keys.push(format!("omdb:{omdb}"));
    }
    for other in &ids.other {
        keys.push(format!("{}:{}", other.source.to_lowercase(), other.id));
    }
    keys
}

fn merge_group(group: &mut TitleGroup, incoming: TitleMatch, index: usize) {
    let mut merged_ids = group.best.extras.external_ids.clone();
    merge_external_ids(&mut merged_ids, &incoming.extras.external_ids);

    let mut merged_aliases = group.best.extras.aliases.clone();
    add_aliases(&mut merged_aliases, &group.best.name);
    add_aliases(&mut merged_aliases, &incoming.name);
    for alias in &incoming.extras.aliases {
        add_aliases(&mut merged_aliases, alias);
    }

    if incoming.global_score > group.best.global_score {
        let mut replacement = incoming;
        replacement.extras.external_ids = merged_ids;
        replacement.extras.aliases = merged_aliases;
        group.best = replacement;
    } else {
        group.best.extras.external_ids = merged_ids;
        group.best.extras.aliases = merged_aliases;
    }

    group.insertion_index = group.insertion_index.min(index);
}

fn add_aliases(aliases: &mut Vec<String>, alias: &str) {
    if alias.trim().is_empty() {
        return;
    }
    if !aliases
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(alias))
    {
        aliases.push(alias.to_string());
    }
}

fn merge_external_ids(target: &mut ExternalIds, incoming: &ExternalIds) {
    if target.imdb.is_none() {
        target.imdb = incoming.imdb.clone();
    }
    if target.tmdb.is_none() {
        target.tmdb = incoming.tmdb.clone();
    }
    if target.tvdb.is_none() {
        target.tvdb = incoming.tvdb.clone();
    }
    if target.tvmaze.is_none() {
        target.tvmaze = incoming.tvmaze.clone();
    }
    if target.anidb.is_none() {
        target.anidb = incoming.anidb.clone();
    }
    if target.omdb.is_none() {
        target.omdb = incoming.omdb.clone();
    }

    for other in &incoming.other {
        if !target
            .other
            .iter()
            .any(|item| item.source.eq_ignore_ascii_case(&other.source) && item.id == other.id)
        {
            target.other.push(ExternalId {
                source: other.source.clone(),
                id: other.id.clone(),
            });
        }
    }
}

fn year_compatible(left: &TitleMatch, right: &TitleMatch) -> bool {
    match (left.year, right.year) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}
