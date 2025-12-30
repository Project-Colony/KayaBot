#[derive(Debug, Clone, Default)]
pub struct ParsedName {
    pub title: Option<String>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub used_numeric_heuristic: bool,
}

pub fn parse_filename(filename: &str) -> ParsedName {
    let basename = std::path::Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(filename);
    let cleaned = strip_bracketed(basename);
    let tokens = clean_tokens(split_tokens(&cleaned));
    if tokens.is_empty() {
        return ParsedName::default();
    }

    let mut title_tokens = Vec::new();
    let mut season = None;
    let mut episode = None;
    let mut year = None;
    let mut used_numeric_heuristic = false;

    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        let lower = token.to_lowercase();

        if let Some((s, e)) = parse_compact_season_episode(&lower) {
            season = Some(s);
            episode = Some(e);
            break;
        }

        if let Some((s, e)) = parse_season_episode_pair(&tokens, index) {
            season = Some(s);
            episode = Some(e);
            break;
        }

        if let Some((s, e)) = parse_named_season_episode(&tokens, index) {
            season = Some(s);
            episode = Some(e);
            break;
        }

        if let Some((s, e, heuristic)) = parse_numeric_episode(&lower) {
            season = Some(s);
            episode = Some(e);
            used_numeric_heuristic = heuristic;
            break;
        }

        if let Some(parsed_year) = parse_year(&lower) {
            year = Some(parsed_year);
            break;
        }

        title_tokens.push(token.clone());
        index += 1;
    }

    let title = if title_tokens.is_empty() {
        None
    } else {
        Some(title_tokens.join(" "))
    };

    ParsedName {
        title,
        season,
        episode,
        year,
        used_numeric_heuristic,
    }
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

fn strip_bracketed(input: &str) -> String {
    let mut output = String::new();
    let mut depth = 0;
    for ch in input.chars() {
        match ch {
            '(' | '[' | '{' => {
                depth += 1;
                output.push(' ');
            }
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
                output.push(' ');
            }
            _ => {
                if depth == 0 {
                    output.push(ch);
                }
            }
        }
    }
    output
}

fn clean_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut filtered: Vec<String> = tokens
        .into_iter()
        .filter(|token| {
            let lower = token.to_lowercase();
            !is_release_tag(&lower) && !is_group_name(&lower)
        })
        .collect();

    if let Some(last) = filtered.last() {
        if looks_like_group_suffix(last) {
            filtered.pop();
        }
    }

    filtered
}

fn is_release_tag(token: &str) -> bool {
    matches!(
        token,
        "1080p"
            | "720p"
            | "480p"
            | "2160p"
            | "4k"
            | "x264"
            | "x265"
            | "h264"
            | "h265"
            | "hevc"
            | "av1"
            | "bluray"
            | "bdrip"
            | "brrip"
            | "webrip"
            | "webdl"
            | "web"
            | "hdrip"
            | "dvdrip"
            | "hdtv"
            | "proper"
            | "repack"
            | "extended"
            | "remastered"
            | "subbed"
            | "dubbed"
            | "multi"
            | "dual"
            | "dts"
            | "aac"
            | "ac3"
            | "ddp"
            | "truehd"
            | "atmos"
            | "hdr"
            | "hdr10"
            | "dv"
            | "sample"
            | "cam"
            | "ts"
            | "tc"
            | "scr"
            | "unrated"
            | "limited"
            | "uncut"
            | "complete"
            | "readnfo"
    )
}

fn is_group_name(token: &str) -> bool {
    matches!(
        token,
        "rarbg"
            | "yify"
            | "yts"
            | "eztv"
            | "rartv"
            | "ettv"
            | "psa"
            | "ntb"
            | "amzn"
            | "nf"
            | "dsnp"
            | "hmax"
            | "hulu"
    )
}

fn looks_like_group_suffix(token: &str) -> bool {
    let is_all_upper = token
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit());
    let has_upper = token.chars().any(|ch| ch.is_ascii_uppercase());
    is_all_upper && has_upper && token.len() <= 6
}

fn parse_compact_season_episode(token: &str) -> Option<(u32, u32)> {
    if let Some(stripped) = token.strip_prefix('s') {
        if let Some((season_part, episode_part)) = stripped.split_once('e') {
            let season = season_part.parse::<u32>().ok()?;
            let episode = episode_part.parse::<u32>().ok()?;
            return Some((season, episode));
        }
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

fn parse_numeric_episode(token: &str) -> Option<(u32, u32, bool)> {
    if token.len() == 3 && token.chars().all(|c| c.is_ascii_digit()) {
        let mut digits = token.chars();
        let season = digits.next()?.to_digit(10)? as u32;
        let episode: u32 = digits.collect::<String>().parse().ok()?;
        return Some((season, episode, true));
    }
    None
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
