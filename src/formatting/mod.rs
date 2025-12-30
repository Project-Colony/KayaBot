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

pub const DEFAULT_SERIES_FORMAT: &str = "{n} {s}x{e} - {t}";
pub const DEFAULT_MOVIE_FORMAT: &str = "{n} ({y})";

pub struct SeriesFormatInput<'a> {
    pub series: &'a str,
    pub season: u32,
    pub episode: u32,
    pub title: Option<&'a str>,
    pub year: Option<u32>,
}

pub struct MovieFormatInput<'a> {
    pub title: &'a str,
    pub year: Option<u32>,
}

pub fn format_series_name(input: SeriesFormatInput<'_>, options: FormatOptions) -> String {
    let template = if options.include_episode_title {
        DEFAULT_SERIES_FORMAT
    } else {
        "{n} {s}x{e}"
    };
    format_series_template(template, input)
}

pub fn format_movie_name(input: MovieFormatInput<'_>, options: FormatOptions) -> String {
    let template = if options.include_year {
        DEFAULT_MOVIE_FORMAT
    } else {
        "{n}"
    };
    format_movie_template(template, input)
}

pub fn format_series_template(template: &str, input: SeriesFormatInput<'_>) -> String {
    let series = sanitize_component(input.series);
    let name = if series.trim().is_empty() {
        "Unknown Series".to_string()
    } else {
        series
    };
    let title = input
        .title
        .map(sanitize_component)
        .filter(|value| !value.trim().is_empty());

    let context = TemplateContext {
        name,
        season: Some(input.season),
        episode: Some(input.episode),
        title,
        year: input.year,
    };

    render_template(template, &context)
}

pub fn format_movie_template(template: &str, input: MovieFormatInput<'_>) -> String {
    let title = sanitize_component(input.title);
    let name = if title.trim().is_empty() {
        "Unknown Title".to_string()
    } else {
        title
    };
    let context = TemplateContext {
        name,
        season: None,
        episode: None,
        title: None,
        year: input.year,
    };

    render_template(template, &context)
}

#[derive(Debug, Clone)]
struct TemplateContext {
    name: String,
    season: Option<u32>,
    episode: Option<u32>,
    title: Option<String>,
    year: Option<u32>,
}

#[derive(Debug, Clone)]
enum TemplateSegment {
    Literal(String),
    Token(String),
}

fn render_template(template: &str, context: &TemplateContext) -> String {
    let segments = parse_template(template);
    let mut output = String::new();
    let mut trim_next = false;
    let mut trim_next_closing = false;

    for segment in segments {
        match segment {
            TemplateSegment::Literal(mut text) => {
                if trim_next {
                    text = trim_leading_separators(&text, trim_next_closing);
                    trim_next = false;
                    trim_next_closing = false;
                }
                output.push_str(&text);
            }
            TemplateSegment::Token(token) => {
                if let Some(value) = resolve_token(&token, context) {
                    output.push_str(&value);
                } else {
                    trim_trailing_separators(&mut output);
                    trim_next = true;
                    trim_next_closing = true;
                }
            }
        }
    }

    collapse_whitespace(output)
}

fn parse_template(template: &str) -> Vec<TemplateSegment> {
    let mut segments = Vec::new();
    let mut buffer = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if !buffer.is_empty() {
                segments.push(TemplateSegment::Literal(buffer.clone()));
                buffer.clear();
            }
            let mut token = String::new();
            while let Some(next) = chars.next() {
                if next == '}' {
                    break;
                }
                token.push(next);
            }
            if token.is_empty() {
                buffer.push('{');
            } else {
                segments.push(TemplateSegment::Token(token));
            }
        } else {
            buffer.push(ch);
        }
    }

    if !buffer.is_empty() {
        segments.push(TemplateSegment::Literal(buffer));
    }

    segments
}

fn resolve_token(token: &str, context: &TemplateContext) -> Option<String> {
    let (key, padding) = split_token_padding(token);
    let normalized = normalize_token_key(key);

    match normalized.as_str() {
        "n" | "name" | "title" | "seriestitle" | "movietitle" => Some(context.name.clone()),
        "t" | "episodetitle" => context.title.clone(),
        "s" | "season" => context
            .season
            .map(|value| pad_number(value, padding)),
        "e" | "episode" => context
            .episode
            .map(|value| pad_number(value, padding)),
        "y" | "year" | "releaseyear" => context.year.map(|value| value.to_string()),
        _ => None,
    }
}

fn split_token_padding(token: &str) -> (&str, Option<usize>) {
    let mut split_index = token.len();
    for (index, ch) in token.char_indices().rev() {
        if ch.is_ascii_digit() {
            split_index = index;
        } else {
            break;
        }
    }
    if split_index == token.len() {
        return (token.trim(), None);
    }
    let (key, digits) = token.split_at(split_index);
    let padding = digits.len();
    if padding == 0 {
        (token.trim(), None)
    } else {
        (key.trim(), Some(padding))
    }
}

fn normalize_token_key(token: &str) -> String {
    token
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_')
        .collect::<String>()
        .to_lowercase()
}

fn pad_number(value: u32, padding: Option<usize>) -> String {
    if let Some(width) = padding {
        format!("{value:0width$}", width = width)
    } else {
        value.to_string()
    }
}

fn trim_trailing_separators(output: &mut String) {
    while let Some(last) = output.chars().last() {
        if last.is_whitespace()
            || matches!(
                last,
                '-' | '–' | '—' | ':' | '.' | '/' | '\\' | '_' | '|' | '(' | '['
            )
        {
            output.pop();
        } else {
            break;
        }
    }
}

fn trim_leading_separators(value: &str, trim_closing: bool) -> String {
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        if ch.is_whitespace()
            || matches!(ch, '-' | '–' | '—' | ':' | '.' | '/' | '\\' | '_' | '|')
            || (trim_closing && matches!(ch, ')' | ']'))
        {
            start = index + ch.len_utf8();
            continue;
        }
        break;
    }
    value[start..].to_string()
}

fn collapse_whitespace(value: String) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_movie_template_with_year() {
        let formatted = format_movie_template(
            "{n} ({y})",
            MovieFormatInput {
                title: "My Movie",
                year: Some(2023),
            },
        );

        assert_eq!(formatted, "My Movie (2023)");
    }

    #[test]
    fn format_movie_template_without_year() {
        let formatted = format_movie_template(
            "{n} ({y})",
            MovieFormatInput {
                title: "My Movie",
                year: None,
            },
        );

        assert_eq!(formatted, "My Movie");
    }

    #[test]
    fn format_series_template_with_title() {
        let formatted = format_series_template(
            "{n} {s}x{e} - {t}",
            SeriesFormatInput {
                series: "My Series",
                season: 1,
                episode: 2,
                title: Some("Pilot"),
                year: None,
            },
        );

        assert_eq!(formatted, "My Series 1x2 - Pilot");
    }

    #[test]
    fn format_series_template_without_title() {
        let formatted = format_series_template(
            "{n} {s}x{e} - {t}",
            SeriesFormatInput {
                series: "My Series",
                season: 1,
                episode: 2,
                title: None,
                year: None,
            },
        );

        assert_eq!(formatted, "My Series 1x2");
    }

    #[test]
    fn format_series_template_with_padding() {
        let formatted = format_series_template(
            "{n} - {s00}x{e00}",
            SeriesFormatInput {
                series: "My Series",
                season: 1,
                episode: 2,
                title: None,
                year: None,
            },
        );

        assert_eq!(formatted, "My Series - 01x02");
    }
}
