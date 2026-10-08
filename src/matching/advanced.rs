use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct NormalizedName {
    #[allow(dead_code)]
    pub original: String,
    pub normalized: String,
    #[allow(dead_code)]
    pub tokens: Vec<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub special_tokens: Vec<String>,
    pub removed_tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RankedCandidate {
    #[allow(dead_code)]
    pub original: String,
    #[allow(dead_code)]
    pub normalized: String,
    pub score: f32,
    #[allow(dead_code)]
    pub justification: Vec<String>,
}

pub fn normalize_name(name: &str) -> NormalizedName {
    let tokens = split_tokens(name);
    let mut removed_tags = Vec::new();
    let mut kept_tokens = Vec::new();
    let mut season = None;
    let mut episode = None;
    let mut year = None;
    let mut special_tokens = Vec::new();

    if let Some((s, e)) = detect_season_episode(&tokens) {
        season = Some(s);
        episode = Some(e);
    }

    for token in tokens {
        let lower = token.to_lowercase();
        if let Some(parsed_year) = parse_year(&lower) {
            year = Some(parsed_year);
            removed_tags.push(lower);
            continue;
        }

        if is_quality_tag(&lower) || is_language_tag(&lower) || is_release_tag(&lower) {
            removed_tags.push(lower);
            continue;
        }

        if is_special_token(&lower) {
            special_tokens.push(lower.clone());
        }

        kept_tokens.push(lower);
    }

    let normalized = kept_tokens.join(" ");

    NormalizedName {
        original: name.to_string(),
        normalized,
        tokens: kept_tokens,
        season,
        episode,
        year,
        special_tokens,
        removed_tags,
    }
}

pub fn rank_candidates(query: &str, candidates: &[String]) -> Vec<RankedCandidate> {
    let query_norm = normalize_name(query);
    let mut ranked: Vec<(usize, RankedCandidate)> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let candidate_norm = normalize_name(candidate);
            let mut justification = Vec::new();

            let jw = jaro_winkler(&query_norm.normalized, &candidate_norm.normalized);
            let lev_sim =
                levenshtein_similarity(&query_norm.normalized, &candidate_norm.normalized);
            let mut score = (jw * 0.6) + (lev_sim * 0.4);

            justification.push(format!("fuzzy similarity jw={jw:.3} lev={lev_sim:.3}"));

            if let (Some(qs), Some(qe), Some(cs), Some(ce)) = (
                query_norm.season,
                query_norm.episode,
                candidate_norm.season,
                candidate_norm.episode,
            ) {
                if qs == cs && qe == ce {
                    score += 0.15;
                    justification.push("season/episode match".to_string());
                } else {
                    score -= 0.2;
                    justification.push("season/episode mismatch".to_string());
                }
            } else if query_norm.season.is_some() || query_norm.episode.is_some() {
                score -= 0.1;
                justification.push("missing season/episode".to_string());
            }

            if let (Some(qy), Some(cy)) = (query_norm.year, candidate_norm.year) {
                if qy == cy {
                    score += 0.1;
                    justification.push("year match".to_string());
                } else {
                    score -= 0.15;
                    justification.push("year mismatch".to_string());
                }
            }

            let query_special: HashSet<_> = query_norm.special_tokens.iter().cloned().collect();
            let candidate_special: HashSet<_> =
                candidate_norm.special_tokens.iter().cloned().collect();
            let special_overlap: Vec<_> = query_special
                .intersection(&candidate_special)
                .cloned()
                .collect();

            if !special_overlap.is_empty() {
                score += 0.05;
                justification.push(format!(
                    "special token overlap: {}",
                    special_overlap.join(", ")
                ));
            } else if !query_special.is_empty() {
                score -= 0.05;
                justification.push("special token mismatch".to_string());
            }

            if !candidate_norm.removed_tags.is_empty() {
                justification.push(format!(
                    "removed tags: {}",
                    candidate_norm.removed_tags.join(", ")
                ));
            }

            let score = score.clamp(0.0, 1.0);

            (
                index,
                RankedCandidate {
                    original: candidate.to_string(),
                    normalized: candidate_norm.normalized,
                    score,
                    justification,
                },
            )
        })
        .collect();

    ranked.sort_by(|(a_idx, a), (b_idx, b)| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_idx.cmp(b_idx))
    });

    ranked.into_iter().map(|(_, item)| item).collect()
}

fn split_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch.is_alphanumeric() {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn detect_season_episode(tokens: &[String]) -> Option<(u32, u32)> {
    for (index, token) in tokens.iter().enumerate() {
        let lower = token.to_lowercase();
        if let Some((season, episode)) = parse_compact_season_episode(&lower) {
            return Some((season, episode));
        }
        if let Some((season, episode)) = parse_season_episode_pair(tokens, index) {
            return Some((season, episode));
        }
        if let Some((season, episode)) = parse_named_season_episode(tokens, index) {
            return Some((season, episode));
        }
    }
    None
}

fn parse_compact_season_episode(token: &str) -> Option<(u32, u32)> {
    if let Some(stripped) = token.strip_prefix('s')
        && let Some((season_part, episode_part)) = stripped.split_once('e')
    {
        let season = season_part.parse::<u32>().ok()?;
        let episode = episode_part.parse::<u32>().ok()?;
        return Some((season, episode));
    }

    if let Some((season_part, episode_part)) = token.split_once('x') {
        let season = season_part.parse::<u32>().ok()?;
        let episode = episode_part.parse::<u32>().ok()?;
        return Some((season, episode));
    }

    None
}

fn parse_season_episode_pair(tokens: &[String], index: usize) -> Option<(u32, u32)> {
    if index + 1 >= tokens.len() {
        return None;
    }
    let first = tokens[index].to_lowercase();
    let second = tokens[index + 1].to_lowercase();
    let season = first.strip_prefix('s')?.parse::<u32>().ok()?;
    let episode = second.strip_prefix('e')?.parse::<u32>().ok()?;
    Some((season, episode))
}

fn parse_named_season_episode(tokens: &[String], index: usize) -> Option<(u32, u32)> {
    if index + 3 >= tokens.len() {
        return None;
    }
    let first = tokens[index].to_lowercase();
    if !matches!(first.as_str(), "season" | "saison") {
        return None;
    }
    let season = tokens[index + 1].parse::<u32>().ok()?;
    let third = tokens[index + 2].to_lowercase();
    if !matches!(third.as_str(), "episode" | "ep" | "e") {
        return None;
    }
    let episode = tokens[index + 3].parse::<u32>().ok()?;
    Some((season, episode))
}

fn parse_year(token: &str) -> Option<u32> {
    if token.len() != 4 || !token.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year = token.parse::<u32>().ok()?;
    if (1900..=2099).contains(&year) {
        Some(year)
    } else {
        None
    }
}

fn is_quality_tag(token: &str) -> bool {
    matches!(
        token,
        "1080p"
            | "2160p"
            | "4k"
            | "720p"
            | "480p"
            | "hdr"
            | "hdr10"
            | "dv"
            | "dolbyvision"
            | "x264"
            | "x265"
            | "h264"
            | "h265"
            | "hevc"
            | "av1"
            | "bluray"
            | "brrip"
            | "dvdrip"
            | "webrip"
            | "webdl"
            | "web"
            | "remux"
            | "uhd"
            | "sd"
            | "hd"
            | "fhd"
    )
}

fn is_language_tag(token: &str) -> bool {
    matches!(
        token,
        "french"
            | "truefrench"
            | "vff"
            | "vfi"
            | "vf"
            | "vostfr"
            | "multi"
            | "eng"
            | "english"
            | "jpn"
            | "japanese"
            | "ita"
            | "italian"
            | "spa"
            | "spanish"
            | "ger"
            | "german"
            | "rus"
            | "russian"
            | "kor"
            | "korean"
            | "chi"
            | "chinese"
    )
}

fn is_release_tag(token: &str) -> bool {
    matches!(
        token,
        "rarbg"
            | "yts"
            | "eztv"
            | "ettv"
            | "amzn"
            | "nf"
            | "hmax"
            | "proper"
            | "repack"
            | "uncut"
            | "extended"
            | "limited"
            | "internal"
            | "dubbed"
            | "subbed"
            | "sub"
            | "dub"
    )
}

fn is_special_token(token: &str) -> bool {
    matches!(
        token,
        "special"
            | "specials"
            | "ova"
            | "pilot"
            | "prologue"
            | "epilogue"
            | "extra"
            | "bonus"
            | "part"
            | "chapter"
            | "complete"
    )
}

fn jaro_winkler(a: &str, b: &str) -> f32 {
    let jaro = jaro_similarity(a, b);
    let prefix_len = a
        .chars()
        .zip(b.chars())
        .take_while(|(ca, cb)| ca == cb)
        .count()
        .min(4) as f32;
    jaro + (prefix_len * 0.1 * (1.0 - jaro))
}

fn jaro_similarity(a: &str, b: &str) -> f32 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() && b_chars.is_empty() {
        return 1.0;
    }
    if a_chars.is_empty() || b_chars.is_empty() {
        return 0.0;
    }

    let match_distance = (a_chars.len().max(b_chars.len()) / 2).saturating_sub(1);

    let mut a_matches = vec![false; a_chars.len()];
    let mut b_matches = vec![false; b_chars.len()];

    let mut matches = 0;

    for (i, &a_char) in a_chars.iter().enumerate() {
        let start = i.saturating_sub(match_distance);
        let end = (i + match_distance + 1).min(b_chars.len());
        for j in start..end {
            if b_matches[j] {
                continue;
            }
            if a_char == b_chars[j] {
                a_matches[i] = true;
                b_matches[j] = true;
                matches += 1;
                break;
            }
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut transpositions = 0;
    let mut b_index = 0;

    for i in 0..a_chars.len() {
        if !a_matches[i] {
            continue;
        }
        while b_index < b_chars.len() && !b_matches[b_index] {
            b_index += 1;
        }
        if b_index < b_chars.len() && a_chars[i] != b_chars[b_index] {
            transpositions += 1;
        }
        b_index += 1;
    }

    let matches_f = matches as f32;
    let transpositions_f = (transpositions as f32) / 2.0;

    ((matches_f / a_chars.len() as f32)
        + (matches_f / b_chars.len() as f32)
        + ((matches_f - transpositions_f) / matches_f))
        / 3.0
}

fn levenshtein_similarity(a: &str, b: &str) -> f32 {
    let distance = levenshtein_distance(a, b) as f32;
    let max_len = a.len().max(b.len()) as f32;
    if max_len == 0.0 {
        1.0
    } else {
        1.0 - (distance / max_len)
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() {
        return b_chars.len();
    }
    if b_chars.is_empty() {
        return a_chars.len();
    }

    let mut costs: Vec<usize> = (0..=b_chars.len()).collect();

    for (i, a_char) in a_chars.iter().enumerate() {
        let mut last_cost = i;
        let mut current_cost;
        costs[0] = i + 1;

        for (j, b_char) in b_chars.iter().enumerate() {
            current_cost = costs[j + 1];
            if a_char == b_char {
                costs[j + 1] = last_cost;
            } else {
                costs[j + 1] = 1 + costs[j].min(last_cost).min(current_cost);
            }
            last_cost = current_cost;
        }
    }

    costs[b_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_strips_tags_and_year() {
        let normalized = normalize_name("My Movie 2019 1080p VOSTFR");

        assert_eq!(normalized.normalized, "my movie");
        assert_eq!(normalized.year, Some(2019));
        assert!(normalized.removed_tags.contains(&"2019".to_string()));
        assert!(normalized.removed_tags.contains(&"1080p".to_string()));
        assert!(normalized.removed_tags.contains(&"vostfr".to_string()));
    }

    #[test]
    fn normalize_name_extracts_season_episode() {
        let normalized = normalize_name("My.Show.S02E03.720p");

        assert_eq!(normalized.season, Some(2));
        assert_eq!(normalized.episode, Some(3));
    }

    #[test]
    fn rank_candidates_prefers_matching_season_episode() {
        let ranked = rank_candidates(
            "My Show S01E02",
            &["My Show S01E03".to_string(), "My Show S01E02".to_string()],
        );

        assert_eq!(ranked[0].original, "My Show S01E02");
        assert!(ranked[0].score >= ranked[1].score);
    }
}
