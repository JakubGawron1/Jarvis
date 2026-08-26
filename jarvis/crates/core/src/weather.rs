use crate::visual::weather_visual;
use jarvis_protocol::{Lang, VisualSpec};

pub struct WeatherReport {
    pub city: String,
    pub temp_c: f32,
    pub wind_kmh: f32,
    pub humidity: u32,
    pub condition: String,
}

impl WeatherReport {
    pub fn spoken(&self, lang: Lang) -> String {
        match lang {
            Lang::Pl => format!(
                "Pogoda dla {}: {}, {:.0} °C, wiatr {:.0} km/h, wilgotność {}%.",
                self.city, self.condition, self.temp_c, self.wind_kmh, self.humidity
            ),
            Lang::En => format!(
                "Weather for {}: {}, {:.0} °C, wind {:.0} km/h, humidity {}%.",
                self.city, self.condition, self.temp_c, self.wind_kmh, self.humidity
            ),
        }
    }

    pub fn visual(&self) -> VisualSpec {
        weather_visual(
            &self.city,
            self.temp_c,
            self.wind_kmh,
            self.humidity,
            &self.condition,
        )
    }
}

pub fn looks_like_weather(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("pogod")
        || t.contains("weather")
        || t.contains("temperatur")
        || t.contains("forecast")
        || t.contains("prognoz")
}

pub fn place_from_prompt(text: &str) -> String {
    let lower = text.to_lowercase();
    for sep in [" dla ", " for ", " in ", " w ", " pogodę ", " pogode ", " weather "] {
        if let Some((_, rest)) = lower.split_once(sep) {
            let token = rest
                .trim()
                .trim_matches(|c: char| matches!(c, '.' | '!' | '?' | ',' | ':' | ';'))
                .trim();
            if token.len() >= 2 {
                return pretty_place(token);
            }
        }
    }
    pretty_place(text.trim())
}

fn pretty_place(raw: &str) -> String {
    let t = raw
        .replace("rudy śląskiej", "Ruda Śląska")
        .replace("rudy slaskiej", "Ruda Śląska")
        .replace("śląskiej", "Śląska")
        .replace("slaskiej", "Śląska");
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => t,
    }
}

pub async fn fetch_weather(place: &str) -> anyhow::Result<WeatherReport> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(8))
        .build()?;
    let geo_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=pl",
        urlencoding(place)
    );
    let geo: serde_json::Value = client.get(&geo_url).send().await?.json().await?;
    let first = geo["results"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| anyhow::anyhow!("place not found"))?;
    let lat = first["latitude"].as_f64().unwrap_or(0.0);
    let lon = first["longitude"].as_f64().unwrap_or(0.0);
    let city = first["name"]
        .as_str()
        .unwrap_or(place)
        .to_string();
    let fx_url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m&wind_speed_unit=kmh"
    );
    let fx: serde_json::Value = client.get(&fx_url).send().await?.json().await?;
    let cur = &fx["current"];
    let code = cur["weather_code"].as_i64().unwrap_or(0) as i32;
    Ok(WeatherReport {
        city,
        temp_c: cur["temperature_2m"].as_f64().unwrap_or(0.0) as f32,
        wind_kmh: cur["wind_speed_10m"].as_f64().unwrap_or(0.0) as f32,
        humidity: cur["relative_humidity_2m"].as_u64().unwrap_or(0) as u32,
        condition: wmo_pl(code),
    })
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .flat_map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                vec![b as char]
            } else if b == b' ' {
                vec!['+']
            } else {
                format!("%{b:02X}").chars().collect()
            }
        })
        .collect()
}

fn wmo_pl(code: i32) -> String {
    match code {
        0 => "bezchmurnie".into(),
        1 | 2 => "lekkie zachmurzenie".into(),
        3 => "pochmurno".into(),
        45 | 48 => "mgła".into(),
        51..=57 => "mżawka".into(),
        61..=67 => "deszcz".into(),
        71..=77 => "śnieg".into(),
        80..=82 => "przelotne opady".into(),
        85 | 86 => "przelotny śnieg".into(),
        95..=99 => "burza".into(),
        _ => "zmienna".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_weather() {
        assert!(looks_like_weather("pokaż pogodę dla rudy Śląskiej"));
        assert!(looks_like_weather("weather in London"));
        assert!(!looks_like_weather("otwórz notatnik"));
    }

    #[test]
    fn extracts_ruda() {
        let p = place_from_prompt("dobra stary pokaż pogodę dla rudy Śląskiej");
        assert!(p.to_lowercase().contains("ruda"));
    }
}
