#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub eyebrow: String,
    pub cover: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub id: i64,
    pub text: String,
    pub position: f64,
    pub active: bool,
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct HomeFeed {
    pub top_picks: Vec<Album>,
    pub recently_played: Vec<Album>,
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Session {
    pub name: String,
}

#[derive(Clone, Debug, Hash, PartialEq)]
pub struct ApiError {
    pub message: String,
}

fn album(id: i64, title: &str, artist: &str, eyebrow: &str) -> Album {
    Album {
        id,
        title: title.into(),
        artist: artist.into(),
        eyebrow: eyebrow.into(),
        cover: cover_path(id),
    }
}

pub fn cover_path(id: i64) -> String {
    format!("{}/assets/cover-{id:02}.png", env!("CARGO_MANIFEST_DIR"))
}

fn catalog() -> Vec<Album> {
    vec![
        album(1, "Velvet Sun", "Mira Vale", "Made for You"),
        album(2, "Liquid Light", "Nova June", "Featuring Nova June"),
        album(3, "After Blue", "The Coastline", "New Release"),
        album(4, "Midnight Bloom", "Aya North", "Made for You"),
        album(5, "Glass Garden", "Lena Field", "New Release"),
        album(6, "Chrome Heart", "Night Static", "Electronic"),
        album(7, "Soft Weather", "Cloud House", "Chill"),
        album(8, "Open Water", "Emerald Sky", "Alternative"),
        album(9, "Amber Signal", "Low Atlas", "Indie"),
    ]
}

pub async fn load_home() -> Result<HomeFeed, ApiError> {
    let albums = catalog();
    Ok(HomeFeed {
        top_picks: albums[..5].to_vec(),
        recently_played: albums,
    })
}

pub async fn authenticate() -> Result<Session, ApiError> {
    Ok(Session {
        name: "Eddy Kim".into(),
    })
}

pub async fn search_catalog(query: String) -> Result<Vec<Album>, ApiError> {
    let query = query.trim().to_lowercase();
    Ok(catalog()
        .into_iter()
        .filter(|album| {
            album.title.to_lowercase().contains(&query)
                || album.artist.to_lowercase().contains(&query)
        })
        .collect())
}

pub async fn adjacent_track(current_title: String, step: i64) -> Result<Album, ApiError> {
    let albums = catalog();
    let current = albums
        .iter()
        .position(|album| album.title == current_title)
        .ok_or_else(|| ApiError {
            message: "The current song is no longer in the mock catalog.".into(),
        })?;
    let next = adjacent_index(albums.len(), current, step);
    Ok(albums[next].clone())
}

pub fn playback_elapsed(progress: f64) -> String {
    format_duration(progress_seconds(progress))
}

pub fn playback_remaining(progress: f64) -> String {
    format!("-{}", format_duration(227 - progress_seconds(progress)))
}

pub fn remember_volume(volume: f64, previous: f64) -> f64 {
    if volume > 0.0 { volume } else { previous }
}

pub fn toggle_mute(volume: f64, unmuted_volume: f64) -> f64 {
    if volume > 0.0 { 0.0 } else { unmuted_volume }
}

pub fn lyrics_for(title: String, progress: f64) -> Vec<LyricLine> {
    let text = [
        title,
        "After blue".into(),
        "Under a velvet sun".into(),
        "Soft weather moving in".into(),
        "An amber signal".into(),
        "Across the open water".into(),
    ];
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 100.0)
    } else {
        0.0
    };
    let active = ((progress / 100.0 * text.len() as f64).floor() as usize).min(text.len() - 1);

    text.into_iter()
        .enumerate()
        .map(|(id, text)| LyricLine {
            id: id as i64,
            text,
            position: id as f64 * 20.0,
            active: id == active,
        })
        .collect()
}

fn progress_seconds(progress: f64) -> u64 {
    if progress.is_finite() {
        (progress.clamp(0.0, 100.0) * 2.27).round() as u64
    } else {
        0
    }
}

fn format_duration(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn adjacent_index(len: usize, current: usize, step: i64) -> usize {
    (current as i64 + step).rem_euclid(len as i64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_search_matches_titles_and_artists() {
        let matches = iced::futures::executor::block_on(search_catalog(" NoVa ".into())).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "Liquid Light");
    }

    #[test]
    fn adjacent_tracks_wrap_at_both_ends() {
        let previous =
            iced::futures::executor::block_on(adjacent_track("Velvet Sun".into(), -1)).unwrap();
        let next =
            iced::futures::executor::block_on(adjacent_track("Amber Signal".into(), 1)).unwrap();

        assert_eq!(previous.title, "Amber Signal");
        assert_eq!(next.title, "Velvet Sun");
        assert_eq!(
            iced::futures::executor::block_on(adjacent_track("Missing".into(), 1))
                .unwrap_err()
                .message,
            "The current song is no longer in the mock catalog."
        );
    }

    #[test]
    fn playback_details_follow_position_and_mute_state() {
        assert_eq!(playback_elapsed(0.0), "0:00");
        assert_eq!(playback_elapsed(50.0), "1:54");
        assert_eq!(playback_remaining(100.0), "-0:00");
        assert_eq!(remember_volume(42.0, 76.0), 42.0);
        assert_eq!(remember_volume(0.0, 42.0), 42.0);
        assert_eq!(toggle_mute(42.0, 42.0), 0.0);
        assert_eq!(toggle_mute(0.0, 42.0), 42.0);
    }

    #[test]
    fn lyrics_follow_the_playhead() {
        let lines = lyrics_for("Liquid Light".into(), 34.0);

        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0].text, "Liquid Light");
        assert!(lines[2].active);
        assert_eq!(lines[3].position, 60.0);
    }
}
