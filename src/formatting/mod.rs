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

pub const DEFAULT_SERIES_FORMAT: &str =
    "~/Media/TV Shows/{Series}/Season {season:02}/{Series} - S{season:02}E{episode:02} - {title}";
pub const DEFAULT_MOVIE_FORMAT: &str = "~/Media/Movies/{title} ({year})/{title} ({year})";

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
    let mut name = format!(
        "~/Media/TV Shows/{series}/Season {:02}/{series} - S{:02}E{:02}",
        input.season, input.season, input.episode
    );

    if options.include_episode_title {
        if let Some(title) = input.title {
            let cleaned = sanitize_component(title);
            name.push_str(&format!(" - {cleaned}"));
        } else {
            name.push_str(&format!(" - Episode {:02}", input.episode));
        }
    }

    name
}

pub fn format_movie_name(input: MovieFormatInput<'_>, options: FormatOptions) -> String {
    let title = sanitize_component(input.title);
    let mut base = title.clone();
    if options.include_year {
        if let Some(year) = input.year {
            base = format!("{base} ({year})");
        }
    }

    format!("~/Media/Movies/{base}/{base}")
}

pub fn sanitize_component(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    for ch in value.chars() {
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
