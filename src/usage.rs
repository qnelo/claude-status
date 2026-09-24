//! Response of `GET https://api.anthropic.com/api/oauth/usage` and the texts the applet shows.

use chrono::{DateTime, Datelike, NaiveDateTime, TimeDelta, Timelike, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Limit {
    pub pct: f32,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct UsageData {
    pub session: Limit,
    pub weekly_all: Limit,
    /// Per-model weekly limits (only Fable today), named as the API returns them.
    pub weekly_models: Vec<(String, Limit)>,
}

// Only `limits` is read: it comes already grouped and carries the model name. The loose keys
// (`five_hour`, `seven_day`, `nimbus_quill`...) repeat the same data under internal names.
#[derive(Deserialize)]
struct Response {
    limits: Vec<ApiLimit>,
}

#[derive(Deserialize)]
struct ApiLimit {
    kind: String,
    percent: f32,
    resets_at: Option<DateTime<Utc>>,
    scope: Option<Scope>,
}

#[derive(Deserialize)]
struct Scope {
    model: Option<Model>,
}

#[derive(Deserialize)]
struct Model {
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    oauth: OAuth,
}

#[derive(Deserialize)]
struct OAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
}

/// OAuth token left by Claude Code. Read-only: refreshing it here would rotate the refresh token
/// and log Claude Code out.
fn read_token() -> Result<String, String> {
    let path = std::env::home_dir()
        .ok_or("No encuentro el directorio HOME")?
        .join(".claude/.credentials.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("No pude leer {}: {e}", path.display()))?;
    // The serde error is not propagated: it may quote parts of the file, token included.
    let creds: Credentials = serde_json::from_str(&raw)
        .map_err(|_| "Las credenciales de Claude Code tienen un formato inesperado".to_string())?;
    Ok(creds.oauth.access_token)
}

pub async fn fetch() -> Result<UsageData, String> {
    let resp = reqwest::Client::new()
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(read_token()?)
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("user-agent", "claude-status-applet")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Sin conexión con la API: {e}"))?;

    match resp.status().as_u16() {
        200 => parse(&resp.text().await.map_err(|e| format!("Respuesta cortada: {e}"))?),
        401 => Err("Token vencido: abre Claude Code para renovarlo".into()),
        429 => Err("La API pidió esperar; reintento en el próximo ciclo".into()),
        s => Err(format!("La API respondió HTTP {s}")),
    }
}

pub fn parse(json: &str) -> Result<UsageData, String> {
    let resp: Response =
        serde_json::from_str(json).map_err(|e| format!("Respuesta inesperada de la API: {e}"))?;

    let (mut session, mut weekly_all, mut weekly_models) = (None, None, Vec::new());
    for l in resp.limits {
        let limit = Limit { pct: l.percent, resets_at: l.resets_at };
        match l.kind.as_str() {
            "session" => session = Some(limit),
            "weekly_all" => weekly_all = Some(limit),
            "weekly_scoped" => {
                let name = l.scope.and_then(|s| s.model).and_then(|m| m.display_name);
                weekly_models.push((name.unwrap_or_else(|| "Otro límite".into()), limit));
            }
            _ => {}
        }
    }

    Ok(UsageData {
        session: session.ok_or("La API no devolvió el límite de sesión")?,
        weekly_all: weekly_all.ok_or("La API no devolvió el límite semanal")?,
        weekly_models,
    })
}

/// "Se restablece en 4 h 12 min", rounded up to the minute.
pub fn resets_in(at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let mins = (at - now).num_seconds().max(0).unsigned_abs().div_ceil(60);
    match (mins / 60, mins % 60) {
        (0, m) => format!("Se restablece en {m} min"),
        (h, 0) => format!("Se restablece en {h} h"),
        (h, m) => format!("Se restablece en {h} h {m} min"),
    }
}

/// "Se restablece el sáb, 1:00 p.m.", with `local` already in local time.
pub fn resets_on(local: NaiveDateTime) -> String {
    const DAYS: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];
    // Round to the nearest minute: the API returns 12:59:59.8 and claude.ai shows 1:00.
    let t = local + TimeDelta::seconds(30);
    let (pm, hour) = t.hour12();
    format!(
        "Se restablece el {}, {hour}:{:02} {}",
        DAYS[t.weekday().num_days_from_monday() as usize],
        t.minute(),
        if pm { "p.m." } else { "a.m." }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // JSON integers, exact in f32
    fn parses_real_response() {
        let u = parse(include_str!("sample.json")).unwrap();
        assert_eq!(u.session.pct, 8.0);
        assert_eq!(u.weekly_all.pct, 44.0);
        assert_eq!(u.weekly_models.len(), 1);
        assert_eq!(u.weekly_models[0].0, "Fable");
        assert_eq!(u.weekly_models[0].1.pct, 30.0);
        assert!(parse(r#"{"limits": []}"#).is_err());
    }

    #[test]
    fn formats_resets() {
        let at: DateTime<Utc> = "2026-09-24T22:39:59.8Z".parse().unwrap();
        assert_eq!(resets_in(at, at - TimeDelta::minutes(252)), "Se restablece en 4 h 12 min");
        assert_eq!(resets_in(at, at - TimeDelta::seconds(90)), "Se restablece en 2 min");
        assert_eq!(resets_in(at, at - TimeDelta::hours(2)), "Se restablece en 2 h");
        assert_eq!(resets_in(at, at + TimeDelta::minutes(5)), "Se restablece en 0 min");

        // 15:59:59.8Z in Chile (UTC-3 in September) is 12:59:59.8 on Saturday.
        let local: NaiveDateTime = "2026-09-26T12:59:59.8".parse().unwrap();
        assert_eq!(resets_on(local), "Se restablece el sáb, 1:00 p.m.");
        let morning: NaiveDateTime = "2026-09-28T09:05:00".parse().unwrap();
        assert_eq!(resets_on(morning), "Se restablece el lun, 9:05 a.m.");
    }
}
