use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::kind::ActionKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentState {
    Active,
    Inactive,
    Intermediate,
    Neutral,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub label: String,
    pub color: String,
    pub state: SegmentState,
}

pub fn hardware_state(states: impl IntoIterator<Item = SegmentState>) -> i32 {
    let mut any = false;
    let mut all_active = true;
    for state in states {
        any = true;
        if state != SegmentState::Active {
            all_active = false;
        }
    }
    if any && all_active {
        0
    } else {
        1
    }
}

pub fn key_image(kind: ActionKind, segments: &[Segment], foreground: &str) -> String {
    let strip = if segments.is_empty() {
        0.0
    } else if segments.len() <= 6 {
        22.0
    } else {
        10.0
    };
    let width = if segments.is_empty() {
        0.0
    } else {
        144.0 / segments.len() as f32
    };
    let mut bars = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let x = width * index as f32;
        let fill = fill_for(segment);
        bars.push_str(&format!(
            r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{strip:.2}" fill="{fill}"/>"#,
            y = 144.0 - strip,
        ));
        if segment.state == SegmentState::Unavailable {
            bars.push_str(&format!(
                r#"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{strip:.2}" fill="url(#hatch)"/>"#,
                y = 144.0 - strip,
            ));
        }
        if segments.len() <= 6 && strip > 0.0 {
            let label = escape(&initial(&segment.label));
            bars.push_str(&format!(
                r#"<text x="{tx:.2}" y="138" text-anchor="middle" font-size="12" font-family="sans-serif" font-weight="700" fill="{foreground}">{label}</text>"#,
                tx = x + width / 2.0,
            ));
        }
    }
    let icon = icon_path(kind);
    let hatch = "#111";
    let background = "#1b1f27";
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 144 144"><defs><pattern id="hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="6" stroke="{hatch}" stroke-width="2"/></pattern></defs><rect width="144" height="144" rx="18" fill="{background}"/><g fill="none" stroke="{foreground}" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" transform="translate(40 28)">{icon}</g>{bars}</svg>"#
    );
    format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    )
}

fn fill_for(segment: &Segment) -> String {
    match segment.state {
        SegmentState::Active | SegmentState::Neutral => segment.color.clone(),
        SegmentState::Inactive => "#2a3140".into(),
        SegmentState::Intermediate => "#d08a2f".into(),
        SegmentState::Unavailable => "#3a241f".into(),
    }
}

fn initial(label: &str) -> String {
    label
        .chars()
        .find(|char| !char.is_whitespace())
        .map(|char| char.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn icon_path(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Program => r#"<rect x="8" y="16" width="48" height="32" rx="4"/>"#,
        ActionKind::Preview => {
            r#"<rect x="8" y="16" width="48" height="32" rx="4"/><path d="M20 32h24"/>"#
        }
        ActionKind::Transition => r#"<path d="M12 40h40M32 16v24"/>"#,
        ActionKind::Stinger => r#"<path d="M16 40l16-24 16 24"/>"#,
        ActionKind::FadeToBlack => r#"<rect x="12" y="16" width="40" height="32"/>"#,
        ActionKind::Overlay => {
            r#"<rect x="8" y="20" width="28" height="20"/><rect x="28" y="12" width="28" height="20"/>"#
        }
        ActionKind::Recording => r#"<circle cx="32" cy="32" r="14"/>"#,
        ActionKind::Streaming => r#"<path d="M8 18h28l10 10V18h10v28H8z"/>"#,
        ActionKind::External => r#"<path d="M16 40V20h32v20M24 20v12h16"/>"#,
        ActionKind::MultiCorder => {
            r#"<rect x="8" y="16" width="18" height="14"/><rect x="30" y="16" width="18" height="14"/><rect x="8" y="34" width="18" height="14"/><rect x="30" y="34" width="18" height="14"/>"#
        }
        ActionKind::Fullscreen => r#"<path d="M12 24V12h12M52 12v12M52 40v12H40M12 40v12h12"/>"#,
        ActionKind::Replay => r#"<path d="M16 32a16 16 0 1 0 4-10M16 14v10h10"/>"#,
        ActionKind::Mute => r#"<path d="M16 24h10l14-10v36L26 40H16zM46 24l12 16M58 24L46 40"/>"#,
        ActionKind::Solo => r#"<circle cx="32" cy="32" r="14"/><path d="M32 22v20"/>"#,
        ActionKind::BusSend => r#"<path d="M12 32h28M32 20l12 12-12 12"/>"#,
        ActionKind::Play => r#"<path d="M20 16l28 16-28 16z"/>"#,
        ActionKind::List => r#"<path d="M16 18h32M16 32h32M16 46h32"/>"#,
        ActionKind::Title => r#"<path d="M12 20h40M20 20v24M44 20v24"/>"#,
        ActionKind::Shortcut => r#"<path d="M16 40l12-24h8l12 24M22 32h20"/>"#,
        ActionKind::Raw => r#"<path d="M20 16l-10 16 10 16M44 16l10 16-10 16"/>"#,
        ActionKind::Volume => r#"<path d="M12 26h8l12-10v36L20 42h-8zM40 24a12 12 0 0 1 0 16"/>"#,
        ActionKind::ReplayJog => {
            r#"<path d="M16 32a16 16 0 1 0 4-10M16 14v10h10M10 52h12M42 52h12"/>"#
        }
        ActionKind::Gain => r#"<path d="M16 48V16h8l24 32V16"/>"#,
        ActionKind::Headphones => {
            r#"<path d="M16 32a16 16 0 1 1 32 0M16 32v12h8V32M40 32v12h8V32"/>"#
        }
        ActionKind::Mixer => {
            r#"<path d="M20 16v32M32 22v26M44 18v30M14 28h12M26 40h12M38 24h12"/>"#
        }
        ActionKind::Rate => r#"<path d="M16 40l16-24 16 24M24 40h16"/>"#,
        ActionKind::ReplaySpeed => {
            r#"<path d="M16 32a16 16 0 1 0 4-10M16 14v10h10M28 32h16l-8-8M36 32l8 8"/>"#
        }
        ActionKind::Position => r#"<path d="M12 32h40M20 24l-8 8 8 8M44 24l8 8-8 8"/>"#,
    }
}

pub const LAYOUT_B1: &str = "$B1";
pub const LAYOUT_C1: &str = "$C1";
pub const LAYOUT_MULTI: &str = "layouts/dial-multi.json";

pub fn dial_icon(kind: ActionKind, foreground: &str) -> String {
    let path = icon_path(kind);
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 64 64"><g fill="none" stroke="{foreground}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round">{path}</g></svg>"#
    );
    format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg.as_bytes())
    )
}

fn indicator(segment: Option<&Segment>, level: Option<f32>, kind: ActionKind) -> serde_json::Value {
    let connected = segment.is_some_and(|item| item.state != SegmentState::Unavailable);
    let color = segment.map(fill_for).unwrap_or_else(|| "#4c8dff".into());
    let Some(level) = level else {
        return serde_json::json!({
            "value": 0,
            "enabled": false,
            "bar_fill_c": color,
            "range": { "min": 0, "max": 100 },
        });
    };
    let value = if kind == ActionKind::ReplayJog {
        if level > 0.5 {
            100
        } else {
            0
        }
    } else {
        (level.clamp(0.0, 1.0) * 100.0).round() as i64
    };
    serde_json::json!({
        "value": value,
        "enabled": connected,
        "bar_fill_c": color,
        "range": { "min": 0, "max": 100 },
    })
}

fn value_label(kind: ActionKind, level: Option<f32>, caption: Option<&str>) -> String {
    if let Some(caption) = caption.filter(|value| !value.is_empty()) {
        return caption.to_string();
    }
    let Some(level) = level else {
        return String::new();
    };
    match kind {
        ActionKind::Volume | ActionKind::Headphones => {
            format!("{}%", (level.clamp(0.0, 1.0) * 100.0).round() as u8)
        }
        ActionKind::ReplayJog => {
            if level > 0.5 {
                "Play".into()
            } else {
                "Pause".into()
            }
        }
        ActionKind::Gain => format!("{:.1} dB", level.clamp(0.0, 1.0) * 24.0),
        ActionKind::Rate | ActionKind::ReplaySpeed => {
            format!("{:.2}x", level.clamp(0.0, 1.0) * 4.0)
        }
        ActionKind::Position => format!("{}%", (level.clamp(0.0, 1.0) * 100.0).round() as u8),
        _ => String::new(),
    }
}

fn dial_title(kind: ActionKind, segment: Option<&Segment>) -> String {
    match kind {
        ActionKind::Volume => segment
            .map(|item| item.label.clone())
            .unwrap_or_else(|| "Volume".into()),
        ActionKind::ReplayJog => "Replay".into(),
        ActionKind::Gain => "Gain".into(),
        ActionKind::Headphones => "HP".into(),
        ActionKind::Mixer => "Mixer".into(),
        ActionKind::Rate => "Rate".into(),
        ActionKind::ReplaySpeed => "Speed".into(),
        ActionKind::Position => "Pos".into(),
        _ => String::new(),
    }
}

/// Layout id and `setFeedback` payload for Stream Deck+ encoders.
pub fn dial_feedback(
    kind: ActionKind,
    segments: &[Segment],
    levels: &[Option<f32>],
    captions: &[String],
    foreground: &str,
) -> (String, serde_json::Value) {
    let icon = dial_icon(kind, foreground);
    match segments.len() {
        0 | 1 => {
            let segment = segments.first();
            let level = levels.first().copied().flatten();
            (
                LAYOUT_B1.to_string(),
                serde_json::json!({
                    "title": dial_title(kind, segment),
                    "value": value_label(kind, level, captions.first().map(String::as_str)),
                    "icon": icon,
                    "indicator": indicator(segment, level, kind),
                }),
            )
        }
        2 => {
            let title = format!(
                "{} · {}",
                value_label(
                    kind,
                    levels.first().copied().flatten(),
                    captions.first().map(String::as_str)
                ),
                value_label(
                    kind,
                    levels.get(1).copied().flatten(),
                    captions.get(1).map(String::as_str)
                ),
            );
            (
                LAYOUT_C1.to_string(),
                serde_json::json!({
                    "title": title,
                    "icon1": icon,
                    "icon2": icon,
                    "indicator1": indicator(segments.first(), levels.first().copied().flatten(), kind),
                    "indicator2": indicator(segments.get(1), levels.get(1).copied().flatten(), kind),
                }),
            )
        }
        _ => {
            let labels: Vec<String> = segments
                .iter()
                .zip(levels.iter().copied().chain(std::iter::repeat(None)))
                .take(4)
                .enumerate()
                .map(|(index, (segment, level))| {
                    format!(
                        "{} {}",
                        initial(&segment.label),
                        value_label(kind, level, captions.get(index).map(String::as_str))
                    )
                })
                .collect();
            let mut payload = serde_json::json!({ "title": labels.join(" · ") });
            for index in 0..4 {
                payload[format!("indicator{}", index + 1)] = match segments.get(index) {
                    Some(segment) => {
                        indicator(Some(segment), levels.get(index).copied().flatten(), kind)
                    }
                    None => serde_json::json!({ "value": 0, "enabled": false }),
                };
            }
            (LAYOUT_MULTI.to_string(), payload)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_state_is_on_only_when_every_segment_is_active() {
        assert_eq!(
            hardware_state([SegmentState::Active, SegmentState::Active]),
            0
        );
        assert_eq!(
            hardware_state([SegmentState::Active, SegmentState::Inactive]),
            1
        );
        assert_eq!(hardware_state([SegmentState::Neutral]), 1);
    }

    #[test]
    fn dial_feedback_uses_b1_items_for_one_instance() {
        let (layout, payload) = dial_feedback(
            ActionKind::Volume,
            &[Segment {
                label: "Local".into(),
                color: "#4c8dff".into(),
                state: SegmentState::Active,
            }],
            &[Some(0.53)],
            &[],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_B1);
        assert_eq!(payload["title"], "Local");
        assert_eq!(payload["value"], "53%");
        assert_eq!(payload["indicator"]["value"], 53);
        assert_eq!(payload["indicator"]["enabled"], true);
        assert_eq!(payload["indicator"]["bar_fill_c"], "#4c8dff");
        assert_eq!(payload["indicator"]["range"]["min"], 0);
        assert_eq!(payload["indicator"]["range"]["max"], 100);
        assert!(payload["icon"]
            .as_str()
            .unwrap()
            .starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn dial_feedback_uses_c1_items_for_two_instances() {
        let (layout, payload) = dial_feedback(
            ActionKind::ReplayJog,
            &[
                Segment {
                    label: "Local".into(),
                    color: "#4c8dff".into(),
                    state: SegmentState::Active,
                },
                Segment {
                    label: "Remote".into(),
                    color: "#d08a2f".into(),
                    state: SegmentState::Inactive,
                },
            ],
            &[Some(1.0), Some(0.0)],
            &[],
            "#f4f7fb",
        );
        assert_eq!(layout, LAYOUT_C1);
        assert_eq!(payload["title"], "Play · Pause");
        assert_eq!(payload["indicator1"]["value"], 100);
        assert_eq!(payload["indicator2"]["value"], 0);
        assert_eq!(payload["indicator2"]["enabled"], true);
    }
}
