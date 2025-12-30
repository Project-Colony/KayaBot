#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    pub include_episode_title: bool,
    pub include_year: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            include_episode_title: true,
            include_year: true,
        }
    }
}

pub const DEFAULT_SERIES_FORMAT: &str = "{Series Title} {Season}x{Episode} - {Episode Title}";
pub const DEFAULT_MOVIE_FORMAT: &str = "{Title} ({Year})";

pub struct SeriesFormatInput<'a> {
    pub series: &'a str,
    pub season: u32,
    pub episode: u32,
    pub title: Option<&'a str>,
}

pub struct MovieFormatInput<'a> {
    pub title: &'a str,
    pub year: Option<u32>,
}

pub fn format_series_name(input: SeriesFormatInput<'_>, options: FormatOptions) -> String {
    let series = sanitize_component(input.series);
    let mut name = format!("{series} {}x{}", input.season, input.episode);

    if options.include_episode_title {
        let cleaned = input
            .title
            .map(sanitize_component)
            .filter(|value| !value.trim().is_empty());
        if let Some(title) = cleaned {
            name.push_str(&format!(" - {title}"));
        }
    }

    name
}

pub fn format_movie_name(input: MovieFormatInput<'_>, options: FormatOptions) -> String {
    let title = sanitize_component(input.title);
    let mut base = if title.trim().is_empty() {
        "Unknown Title".to_string()
    } else {
        title
    };
    if options.include_year {
        if let Some(year) = input.year {
            base = format!("{base} ({year})");
        }
    }

    base
}

pub fn sanitize_component(value: &str) -> String {
    let normalized = normalize_title(value);
    let mut cleaned = String::with_capacity(normalized.len());
    for ch in normalized.chars() {
        if matches!(
            ch,
            '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>'
        ) {
            cleaned.push('_');
        } else if ch.is_control() {
            cleaned.push('_');
        } else {
            cleaned.push(ch);
        }
    }
    cleaned.trim().to_string()
}

fn normalize_title(value: &str) -> String {
    let mut buffer = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            buffer.push(ch);
        } else {
            buffer.push(' ');
        }
    }

    let tags = [
        "vf",
        "vff",
        "vfi",
        "vostfr",
        "truefrench",
        "multi",
        "1080p",
        "720p",
        "2160p",
        "480p",
        "webrip",
        "webdl",
        "web-dl",
        "bluray",
        "brrip",
        "hdrip",
        "hdtv",
        "dvdrip",
        "x264",
        "x265",
        "h264",
        "h265",
        "aac",
        "dts",
        "truehd",
    ];

    buffer
        .split_whitespace()
        .filter(|token| !is_tag_token(token, &tags))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_tag_token(token: &str, tags: &[&str]) -> bool {
    let lower = token.to_lowercase();
    if tags.contains(&lower.as_str()) {
        return true;
    }
    let bytes = lower.as_bytes();
    if bytes.len() >= 4 && bytes[0] == b's' && bytes.contains(&b'e') {
        let mut has_digit = false;
        for b in bytes {
            if b.is_ascii_digit() {
                has_digit = true;
                break;
            }
        }
        if has_digit {
            return true;
        }
    }
    if let Some((left, right)) = lower.split_once('x') {
        if !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|c| c.is_ascii_digit())
            && right.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    false
}
