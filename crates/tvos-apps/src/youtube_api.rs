//! YouTube Data API v3 client (metadata only: the API does not stream video).
//!
//! The API key is never compiled in. It is read, in this order, from `$TVOS_YOUTUBE_KEY`,
//! `/etc/tvos/youtube.key` (on the device) or `.secrets/youtube_api_key` (development).
//! It is sent in the `X-Goog-Api-Key` header, never in the URL.

use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub views: String,
    pub duration: String,
}

pub fn api_key() -> Option<String> {
    let from_env = std::env::var("TVOS_YOUTUBE_KEY").ok();
    let key = from_env
        .or_else(|| std::fs::read_to_string("/etc/tvos/youtube.key").ok())
        .or_else(|| std::fs::read_to_string(".secrets/youtube_api_key").ok())?;
    let key = key.trim().to_string();
    (!key.is_empty()).then_some(key)
}

/// Most popular videos in Italy.
pub fn fetch_trending(key: &str) -> Result<Vec<Video>, String> {
    let url = "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails,statistics\
               &chart=mostPopular&regionCode=IT&maxResults=12\
               &fields=items(id,snippet(title,channelTitle),contentDetails/duration,statistics/viewCount)";
    parse_videos(&http_get(url, key)?)
}

#[cfg(feature = "net")]
fn http_get(url: &str, key: &str) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let mut resp = agent.get(url).header("X-Goog-Api-Key", key).call().map_err(|e| match e {
        ureq::Error::StatusCode(403) => "Chiave non valida o quota esaurita (HTTP 403)".to_string(),
        ureq::Error::StatusCode(c) => format!("Errore del servizio YouTube (HTTP {c})"),
        other => format!("Rete non raggiungibile: {other}"),
    })?;
    resp.body_mut().read_to_string().map_err(|e| format!("Risposta illeggibile: {e}"))
}

#[cfg(not(feature = "net"))]
fn http_get(_url: &str, _key: &str) -> Result<String, String> {
    let _ = Duration::from_secs(0);
    Err("Rete non disponibile in questa build".to_string())
}

pub fn parse_videos(body: &str) -> Result<Vec<Video>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("JSON non valido: {e}"))?;
    let items = v["items"].as_array().ok_or("Risposta senza elenco video")?;
    Ok(items
        .iter()
        .filter_map(|it| {
            Some(Video {
                id: it["id"].as_str()?.to_string(),
                title: it["snippet"]["title"].as_str()?.to_string(),
                channel: it["snippet"]["channelTitle"].as_str().unwrap_or("").to_string(),
                views: format_views(it["statistics"]["viewCount"].as_str().and_then(|n| n.parse().ok())),
                duration: iso_duration(it["contentDetails"]["duration"].as_str().unwrap_or("")),
            })
        })
        .collect())
}

/// "PT1H2M3S" -> "1:02:03", "PT4M17S" -> "4:17", "P0D" (live) -> "LIVE"
pub fn iso_duration(s: &str) -> String {
    let Some(rest) = s.strip_prefix("PT") else { return "LIVE".into() };
    let (mut h, mut m, mut sec, mut num) = (0u32, 0u32, 0u32, 0u32);
    for c in rest.chars() {
        match c {
            '0'..='9' => num = num * 10 + c.to_digit(10).unwrap(),
            'H' => (h, num) = (num, 0),
            'M' => (m, num) = (num, 0),
            'S' => (sec, num) = (num, 0),
            _ => {}
        }
    }
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

/// 2373603 -> "2,4 Mln visualizzazioni"
pub fn format_views(n: Option<u64>) -> String {
    match n {
        None => String::new(),
        Some(n) if n >= 1_000_000 => format!("{:.1} Mln visualizzazioni", n as f64 / 1e6).replace('.', ","),
        Some(n) if n >= 1_000 => format!("{} Mila visualizzazioni", n / 1_000),
        Some(n) => format!("{n} visualizzazioni"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(iso_duration("PT4M17S"), "4:17");
        assert_eq!(iso_duration("PT1H2M3S"), "1:02:03");
        assert_eq!(iso_duration("PT31S"), "0:31");
        assert_eq!(iso_duration("PT1H"), "1:00:00");
        assert_eq!(iso_duration("P0D"), "LIVE");
    }

    #[test]
    fn views() {
        assert_eq!(format_views(Some(2_373_603)), "2,4 Mln visualizzazioni");
        assert_eq!(format_views(Some(377_953)), "377 Mila visualizzazioni");
        assert_eq!(format_views(Some(12)), "12 visualizzazioni");
        assert_eq!(format_views(None), "");
    }

    #[test]
    fn parses_api_response() {
        let body = r#"{"items":[{"id":"abc","snippet":{"title":"Titolo","channelTitle":"Canale"},
            "contentDetails":{"duration":"PT3M5S"},"statistics":{"viewCount":"1500"}},
            {"snippet":{"title":"senza id"}}]}"#;
        let v = parse_videos(body).unwrap();
        assert_eq!(v.len(), 1); // the item without id is skipped
        assert_eq!(v[0].duration, "3:05");
        assert_eq!(v[0].views, "1 Mila visualizzazioni");
        assert!(parse_videos("{}").is_err());
    }
}
