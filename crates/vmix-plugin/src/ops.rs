use urlencoding::encode;
use vmix_pool::{
    acts_preview_name, acts_program_name, tcp_mix_value, Command, VmixState,
};

use crate::contracts::ActionParams;
use crate::kind::ActionKind;
use crate::render::SegmentState;

pub fn event_affects(kind: ActionKind, name: &str) -> bool {
    match kind {
        ActionKind::Program => name == "Input" || name.starts_with("InputMix"),
        ActionKind::Preview => name == "InputPreview" || name.starts_with("InputPreviewMix"),
        ActionKind::Overlay => name.starts_with("Overlay"),
        ActionKind::FadeToBlack => name == "FadeToBlack",
        ActionKind::Recording => name == "Recording",
        ActionKind::Streaming => name == "Streaming",
        ActionKind::External => name == "External",
        ActionKind::Fullscreen => name == "Fullscreen",
        ActionKind::Replay => name == "ReplayPlaying",
        ActionKind::Mute => {
            name == "InputAudio"
                || name == "MasterAudio"
                || (name.starts_with("Bus") && name.ends_with("Audio") && !name.starts_with("Input"))
        }
        ActionKind::Solo => name == "InputSolo" || name.ends_with("Solo"),
        ActionKind::BusSend => name.starts_with("InputBus") || name == "InputMasterAudio",
        ActionKind::Play => name == "InputPlaying",
        ActionKind::Volume => name.ends_with("Volume"),
        ActionKind::MultiCorder | ActionKind::List | ActionKind::Title => false,
        ActionKind::Transition | ActionKind::Stinger | ActionKind::Shortcut | ActionKind::Raw => {
            false
        }
    }
}

pub fn segment_for(
    kind: ActionKind,
    params: &ActionParams,
    state: &VmixState,
    connected: bool,
) -> SegmentState {
    if !connected {
        return SegmentState::Unavailable;
    }
    if needs_mix(kind) && !state.mix_available(params.mix) {
        return SegmentState::Unavailable;
    }
    match kind {
        ActionKind::Program => {
            lit_input(state.program.get(&params.mix).copied(), state, &params.input)
        }
        ActionKind::Preview => {
            lit_input(state.preview.get(&params.mix).copied(), state, &params.input)
        }
        ActionKind::Overlay => {
            let current = state.overlays.get(&params.overlay.clamp(1, 8)).copied();
            if params.input.trim().is_empty() {
                return if current.is_some() {
                    SegmentState::Active
                } else {
                    SegmentState::Inactive
                };
            }
            lit_input(current, state, &params.input)
        }
        ActionKind::FadeToBlack => flag(state.fade_to_black),
        ActionKind::Recording => flag(state.recording),
        ActionKind::Streaming => flag(state.streaming),
        ActionKind::External => flag(state.external),
        ActionKind::Fullscreen => flag(state.fullscreen),
        ActionKind::MultiCorder => flag(state.multi_corder),
        ActionKind::Replay => flag(state.replay_playing),
        ActionKind::Mute => match audible(state, params) {
            Some(false) => SegmentState::Active,
            _ => SegmentState::Inactive,
        },
        ActionKind::Solo => flag(solo_on(state, params)),
        ActionKind::BusSend => flag(bus_send_on(state, params)),
        ActionKind::Play => {
            let Some(input) = state.resolve_input(&params.input) else {
                return SegmentState::Inactive;
            };
            flag(state.input_playing.get(&input).copied().unwrap_or(false))
        }
        ActionKind::List => {
            let Some(input) = state.resolve_input(&params.input) else {
                return SegmentState::Inactive;
            };
            let current = state
                .inputs
                .iter()
                .find(|item| item.number == input)
                .and_then(|item| item.selected_index.clone());
            if params.index.trim().is_empty() {
                return SegmentState::Neutral;
            }
            if current.as_deref() == Some(params.index.trim()) {
                SegmentState::Active
            } else {
                SegmentState::Inactive
            }
        }
        ActionKind::Transition
        | ActionKind::Stinger
        | ActionKind::Title
        | ActionKind::Shortcut
        | ActionKind::Raw
        | ActionKind::Volume => SegmentState::Neutral,
    }
}

pub fn title_for(kind: ActionKind, params: &ActionParams, state: &VmixState) -> String {
    match kind {
        ActionKind::Volume => level_to_percent(volume_level(state, params)).to_string(),
        ActionKind::Shortcut => shortcut_name(params),
        ActionKind::Program | ActionKind::Preview | ActionKind::Play | ActionKind::Mute => {
            params.input.clone()
        }
        ActionKind::Overlay => format!("OL{}", params.overlay.clamp(1, 8)),
        ActionKind::Stinger => format!("ST{}", params.stinger.clamp(1, 8)),
        ActionKind::List => state
            .resolve_input(&params.input)
            .and_then(|input| {
                state
                    .inputs
                    .iter()
                    .find(|item| item.number == input)
                    .and_then(|item| item.selected_index.clone())
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn command_for(kind: ActionKind, params: &ActionParams) -> Option<Command> {
    let function = match kind {
        ActionKind::Program => "ActiveInput".to_string(),
        ActionKind::Preview => "PreviewInput".to_string(),
        ActionKind::Transition => transition_function(params),
        ActionKind::Stinger => format!("Stinger{}", params.stinger.clamp(1, 8)),
        ActionKind::FadeToBlack => "FadeToBlack".into(),
        ActionKind::Overlay => overlay_function(params),
        ActionKind::Recording => "StartStopRecording".into(),
        ActionKind::Streaming => "StartStopStreaming".into(),
        ActionKind::External => "StartStopExternal".into(),
        ActionKind::MultiCorder => "StartStopMultiCorder".into(),
        ActionKind::Fullscreen => "Fullscreen".into(),
        ActionKind::Replay => replay_function(params),
        ActionKind::Mute => mute_function(params),
        ActionKind::Solo => solo_function(params),
        ActionKind::BusSend => "AudioBus".into(),
        ActionKind::Play => "PlayPause".into(),
        ActionKind::List => list_function(params),
        ActionKind::Title => title_function(params),
        ActionKind::Shortcut => {
            let (name, query) = shortcut_parts(params)?;
            return Some(Command::Function { name, query });
        }
        ActionKind::Raw => {
            return if params.raw.trim().is_empty() {
                None
            } else {
                Some(Command::Raw(params.raw.clone()))
            };
        }
        ActionKind::Volume => return None,
    };
    Some(Command::Function {
        name: function,
        query: query_for(kind, params),
    })
}

pub fn volume_command(params: &ActionParams, percent: u8) -> Command {
    let value = percent.to_string();
    let (name, query) = match params.audio_target.as_str() {
        "master" => ("SetMasterVolume".to_string(), Some(format!("Value={value}"))),
        "bus" => (
            format!("SetBus{}Volume", bus_letter(params)),
            Some(format!("Value={value}")),
        ),
        _ => (
            "SetVolume".to_string(),
            Some(join_query(&[("Value", value.as_str()), ("Input", params.input.trim())])),
        ),
    };
    Command::Function {
        name,
        query: query.filter(|query| !query.is_empty()),
    }
}

pub fn mute_command(params: &ActionParams) -> Command {
    Command::Function {
        name: mute_function(params),
        query: query_for(ActionKind::Mute, params),
    }
}

/// ACTS volume is 0–1. Shortcuts and the dial use 0–100.
pub fn level_to_percent(level: f32) -> u8 {
    (level.clamp(0.0, 1.0) * 100.0).round().clamp(0.0, 100.0) as u8
}

pub fn adjusted_percent(level: f32, ticks: i32, step: f32) -> u8 {
    let current = level_to_percent(level) as f32;
    let step = if step <= 0.0 { 1.0 } else { step };
    (current + ticks as f32 * step).round().clamp(0.0, 100.0) as u8
}

pub fn volume_level(state: &VmixState, params: &ActionParams) -> f32 {
    match params.audio_target.as_str() {
        "master" => state.master_volume,
        "bus" => state
            .bus_volume
            .get(&bus_letter(params))
            .copied()
            .unwrap_or(0.0),
        _ => state
            .resolve_input(&params.input)
            .and_then(|input| state.input_volume.get(&input).copied())
            .unwrap_or(0.0),
    }
}

fn needs_mix(kind: ActionKind) -> bool {
    matches!(
        kind,
        ActionKind::Program | ActionKind::Preview | ActionKind::Stinger | ActionKind::Overlay
    )
}

fn flag(on: bool) -> SegmentState {
    if on {
        SegmentState::Active
    } else {
        SegmentState::Inactive
    }
}

fn lit_input(current: Option<u16>, state: &VmixState, spec: &str) -> SegmentState {
    let Some(wanted) = state.resolve_input(spec) else {
        return SegmentState::Inactive;
    };
    if current == Some(wanted) {
        SegmentState::Active
    } else {
        SegmentState::Inactive
    }
}

fn audible(state: &VmixState, params: &ActionParams) -> Option<bool> {
    match params.audio_target.as_str() {
        "master" => Some(state.master_audio),
        "bus" => state.bus_audio.get(&bus_letter(params)).copied(),
        _ => {
            let input = state.resolve_input(&params.input)?;
            state.input_audio.get(&input).copied()
        }
    }
}

fn solo_on(state: &VmixState, params: &ActionParams) -> bool {
    if params.audio_target == "bus" {
        return state.bus_solo.get(&bus_letter(params)).copied().unwrap_or(false);
    }
    state
        .resolve_input(&params.input)
        .and_then(|input| state.input_solo.get(&input).copied())
        .unwrap_or(false)
}

fn bus_send_on(state: &VmixState, params: &ActionParams) -> bool {
    let Some(input) = state.resolve_input(&params.input) else {
        return false;
    };
    let bus = bus_letter(params);
    if bus == 'M' {
        return state.input_master_audio.get(&input).copied().unwrap_or(false);
    }
    state.input_bus.get(&(input, bus)).copied().unwrap_or(false)
}

fn bus_letter(params: &ActionParams) -> char {
    params
        .bus
        .chars()
        .find(|char| char.is_ascii_alphabetic())
        .map(|char| char.to_ascii_uppercase())
        .filter(|char| matches!(char, 'A'..='G' | 'M'))
        .unwrap_or('A')
}

fn transition_function(params: &ActionParams) -> String {
    let effect = params.effect.trim();
    if effect.is_empty() {
        "Cut".into()
    } else {
        effect.to_string()
    }
}

fn overlay_function(params: &ActionParams) -> String {
    let channel = params.overlay.clamp(1, 8);
    match params.overlay_mode.as_str() {
        "in" => format!("OverlayInput{channel}In"),
        "out" => format!("OverlayInput{channel}Out"),
        "off" => format!("OverlayInput{channel}Off"),
        "zoom" => format!("OverlayInput{channel}Zoom"),
        "last" => format!("OverlayInput{channel}Last"),
        "preview" => format!("PreviewOverlayInput{channel}"),
        _ => format!("OverlayInput{channel}"),
    }
}

fn replay_function(params: &ActionParams) -> String {
    match params.replay_action.as_str() {
        "pause" => "ReplayPause".into(),
        "markin" => "ReplayMarkIn".into(),
        "markout" => "ReplayMarkOut".into(),
        "markinout" => "ReplayMarkInOut".into(),
        "channel" => {
            if params.channel.eq_ignore_ascii_case("b") {
                "ReplaySelectChannelB".into()
            } else {
                "ReplaySelectChannelA".into()
            }
        }
        _ => "ReplayPlayPause".into(),
    }
}

fn mute_function(params: &ActionParams) -> String {
    match params.audio_target.as_str() {
        "master" => "MasterAudio".into(),
        "bus" => format!("Bus{}Audio", bus_letter(params)),
        _ => "Audio".into(),
    }
}

fn solo_function(params: &ActionParams) -> String {
    if params.audio_target == "bus" {
        format!("Bus{}Solo", bus_letter(params))
    } else {
        "Solo".into()
    }
}

fn list_function(params: &ActionParams) -> String {
    match params.list_action.as_str() {
        "previous" => "PreviousItem".into(),
        "select" => "SelectIndex".into(),
        _ => "NextItem".into(),
    }
}

fn title_function(params: &ActionParams) -> String {
    match params.title_action.as_str() {
        "setimage" => "SetImage".into(),
        "visible" => "SetTextVisibleOn".into(),
        _ => "SetText".into(),
    }
}

fn query_for(kind: ActionKind, params: &ActionParams) -> Option<String> {
    let input = params.input.trim();
    let mix = tcp_mix_value(params.mix).map(|value| value.to_string());
    let query = match kind {
        ActionKind::Program | ActionKind::Preview | ActionKind::Stinger => {
            join_query(&[("Input", input), ("Mix", mix.as_deref().unwrap_or(""))])
        }
        ActionKind::Transition => {
            if is_transition_button(&params.effect) {
                String::new()
            } else {
                join_query(&[
                    ("Input", input),
                    ("Mix", mix.as_deref().unwrap_or("")),
                    ("Duration", params.duration_ms.trim()),
                ])
            }
        }
        ActionKind::Overlay => match params.overlay_mode.as_str() {
            "out" | "off" | "zoom" => String::new(),
            "last" => join_query(&[("Mix", mix.as_deref().unwrap_or(""))]),
            "preview" => join_query(&[("Input", input)]),
            _ => join_query(&[("Input", input), ("Mix", mix.as_deref().unwrap_or(""))]),
        },
        ActionKind::Mute | ActionKind::Play => {
            if params.audio_target == "input" || kind == ActionKind::Play {
                join_query(&[("Input", input)])
            } else {
                String::new()
            }
        }
        ActionKind::Solo => {
            if params.audio_target == "bus" {
                String::new()
            } else {
                join_query(&[("Input", input)])
            }
        }
        ActionKind::BusSend => join_query(&[("Value", &bus_letter(params).to_string()), ("Input", input)]),
        ActionKind::List => match params.list_action.as_str() {
            "select" => join_query(&[("Value", params.index.trim()), ("Input", input)]),
            _ => join_query(&[("Input", input)]),
        },
        ActionKind::Title => {
            let mut parts = Vec::new();
            if params.title_action != "visible" && !params.value.is_empty() {
                parts.push(format!("Value={}", encode(params.value.trim())));
            }
            if !input.is_empty() {
                parts.push(format!("Input={input}"));
            }
            if !params.selected_name.trim().is_empty() {
                parts.push(format!("SelectedName={}", encode(params.selected_name.trim())));
            }
            parts.join("&")
        }
        ActionKind::Shortcut => shortcut_parts(params).and_then(|(_, query)| query).unwrap_or_default(),
        ActionKind::Replay => {
            if params.replay_action == "channel" || params.channel.trim().is_empty() {
                String::new()
            } else {
                join_query(&[("Channel", params.channel.trim())])
            }
        }
        _ => String::new(),
    };
    if query.is_empty() {
        None
    } else {
        Some(query)
    }
}

fn shortcut_name(params: &ActionParams) -> String {
    shortcut_parts(params)
        .map(|(name, _)| name)
        .unwrap_or_default()
}

/// A shortcut is one query string, the same shape as the vMix web API:
/// `Function=SetText&Input=1&Value=hello world`.
/// Older settings that stored the name and fields separately still send.
fn shortcut_parts(params: &ActionParams) -> Option<(String, Option<String>)> {
    let line = params.function_name.trim();
    if shortcut_line_is_complete(line) {
        let (name, query) = split_shortcut_line(line)?;
        if name.is_empty() {
            return None;
        }
        return Some((name, if query.is_empty() { None } else { Some(query) }));
    }
    if line.is_empty() {
        return None;
    }
    let query = shortcut_query(params);
    Some((line.to_string(), if query.is_empty() { None } else { Some(query) }))
}

fn shortcut_line_is_complete(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("function=")
        || lower.starts_with("function ")
        || line.contains('&')
        || line.contains('?')
}

fn split_shortcut_line(line: &str) -> Option<(String, String)> {
    let body = query_body(line.trim());
    let body = strip_function_word(body);
    let (head, tail) = split_shortcut_head(body);
    let mut name = String::new();
    let mut pairs = Vec::new();
    take_shortcut_piece(&mut name, &mut pairs, head);
    for piece in tail.split('&') {
        take_shortcut_piece(&mut name, &mut pairs, piece);
    }
    if name.is_empty() {
        return None;
    }
    let query = pairs
        .into_iter()
        .filter(|(key, _)| !key.is_empty())
        .map(|(key, value)| format!("{key}={}", encode_shortcut_value(&value)))
        .collect::<Vec<_>>()
        .join("&");
    Some((name, query))
}

fn query_body(line: &str) -> &str {
    if line.contains("://") {
        return line.split_once('?').map(|(_, query)| query).unwrap_or("");
    }
    line.trim_start_matches('?')
}

fn strip_function_word(body: &str) -> &str {
    let rest = body.trim();
    let prefix = "function ";
    if rest.len() >= prefix.len() && rest[..prefix.len()].eq_ignore_ascii_case(prefix) {
        rest[prefix.len()..].trim()
    } else {
        rest
    }
}

fn split_shortcut_head(body: &str) -> (&str, &str) {
    if let Some((head, tail)) = body.split_once('&') {
        return (head, tail);
    }
    if let Some((head, tail)) = body.split_once('?') {
        return (head, tail);
    }
    if let Some((head, tail)) = body.split_once(char::is_whitespace) {
        if !head.contains('=') {
            return (head.trim(), tail.trim());
        }
    }
    (body.trim(), "")
}

fn take_shortcut_piece(name: &mut String, pairs: &mut Vec<(String, String)>, piece: &str) {
    let piece = piece.trim();
    if piece.is_empty() {
        return;
    }
    match piece.split_once('=') {
        Some((key, value)) if key.eq_ignore_ascii_case("Function") => {
            if name.is_empty() {
                *name = value.trim().to_string();
            }
        }
        Some((key, value)) => pairs.push((key.trim().to_string(), value.trim().to_string())),
        None if name.is_empty() => *name = piece.to_string(),
        None => {}
    }
}

fn encode_shortcut_value(value: &str) -> String {
    let decoded = urlencoding::decode(value).unwrap_or(std::borrow::Cow::Borrowed(value));
    encode(decoded.as_ref()).into_owned()
}

fn shortcut_query(params: &ActionParams) -> String {
    let mix = tcp_mix_value(params.mix).map(|value| value.to_string());
    let mut query = join_query(&[
        ("Input", params.input.trim()),
        ("Value", &encode(params.value.trim()).into_owned()),
        ("Channel", params.channel.trim()),
        ("Mix", mix.as_deref().unwrap_or("")),
        ("Duration", params.duration_ms.trim()),
    ]);
    let extra = params.extra.trim().trim_start_matches('&');
    if !extra.is_empty() {
        if !query.is_empty() {
            query.push('&');
        }
        query.push_str(extra);
    }
    query
}

fn is_transition_button(effect: &str) -> bool {
    let effect = effect.trim();
    let Some(rest) = effect.strip_prefix("Transition") else {
        return false;
    };
    matches!(rest.parse::<u8>(), Ok(1..=4))
}

fn join_query(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Names the feedback path should watch. Exposed for tests.
pub fn watched_program_activator(mix: u8) -> String {
    acts_program_name(mix)
}

pub fn watched_preview_activator(mix: u8) -> String {
    acts_preview_name(mix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vmix_pool::VmixState;

    fn loaded() -> VmixState {
        let mut state = VmixState::default();
        state.program.insert(0, 1);
        state.preview.insert(0, 2);
        state.program.insert(2, 2);
        state.mixes_present.insert(0);
        state.mixes_present.insert(2);
        state.input_audio.insert(1, true);
        state
    }

    #[test]
    fn mix_two_sends_mix_equals_one_and_watches_input_mix_two() {
        let params = ActionParams {
            input: "1".into(),
            mix: 2,
            ..ActionParams::default()
        };
        let Command::Function { name, query } = command_for(ActionKind::Program, &params).unwrap() else {
            panic!("function");
        };
        assert_eq!(name, "ActiveInput");
        assert_eq!(query.as_deref(), Some("Input=1&Mix=1"));
        assert_eq!(watched_program_activator(2), "InputMix2");
        assert_eq!(watched_program_activator(0), "Input");
        assert_eq!(watched_program_activator(16), "InputMix16");
        let mix16 = ActionParams {
            input: "1".into(),
            mix: 16,
            ..ActionParams::default()
        };
        let Command::Function { query, .. } = command_for(ActionKind::Program, &mix16).unwrap() else {
            panic!("function");
        };
        assert_eq!(query.as_deref(), Some("Input=1&Mix=15"));
    }

    #[test]
    fn overlay_eight_lights_only_for_the_matching_input() {
        let mut state = loaded();
        state.overlays.insert(8, 4);
        let matched = ActionParams {
            overlay: 8,
            input: "4".into(),
            ..ActionParams::default()
        };
        let other = ActionParams {
            overlay: 8,
            input: "1".into(),
            ..ActionParams::default()
        };
        assert_eq!(
            segment_for(ActionKind::Overlay, &matched, &state, true),
            SegmentState::Active
        );
        assert_eq!(
            segment_for(ActionKind::Overlay, &other, &state, true),
            SegmentState::Inactive
        );
        assert_eq!(
            segment_for(ActionKind::Overlay, &matched, &state, false),
            SegmentState::Unavailable
        );
    }

    #[test]
    fn mute_is_the_inverse_of_input_audio() {
        let mut state = loaded();
        let params = ActionParams {
            input: "1".into(),
            audio_target: "input".into(),
            ..ActionParams::default()
        };
        state.input_audio.insert(1, true);
        assert_eq!(
            segment_for(ActionKind::Mute, &params, &state, true),
            SegmentState::Inactive
        );
        state.input_audio.insert(1, false);
        assert_eq!(
            segment_for(ActionKind::Mute, &params, &state, true),
            SegmentState::Active
        );
    }

    #[test]
    fn shortcut_encodes_spaces_and_keeps_the_function_name() {
        let params = ActionParams {
            function_name: "SetText".into(),
            input: "1".into(),
            value: "hello world".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = command_for(ActionKind::Shortcut, &params).unwrap() else {
            panic!("function");
        };
        assert_eq!(name, "SetText");
        assert_eq!(query.as_deref(), Some("Input=1&Value=hello%20world"));
        let pasted = ActionParams {
            function_name: "http://127.0.0.1:8088/api/?Function=SetText&Input=1&Value=hello%20world&Mix=1".into(),
            input: "9".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = command_for(ActionKind::Shortcut, &pasted).unwrap() else {
            panic!("function");
        };
        assert_eq!(name, "SetText");
        assert_eq!(query.as_deref(), Some("Input=1&Value=hello%20world&Mix=1"));
        assert_eq!(shortcut_name(&pasted), "SetText");
    }

    #[test]
    fn volume_round_trips_between_acts_fraction_and_percent() {
        assert_eq!(level_to_percent(0.5), 50);
        assert_eq!(level_to_percent(0.0), 0);
        assert_eq!(level_to_percent(1.0), 100);
        assert_eq!(adjusted_percent(0.5, 3, 1.0), 53);
        let params = ActionParams {
            input: "1".into(),
            audio_target: "input".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = volume_command(&params, 50) else {
            panic!("function");
        };
        assert_eq!(name, "SetVolume");
        assert_eq!(query.as_deref(), Some("Value=50&Input=1"));
    }

    #[test]
    fn missing_aux_mix_is_unavailable() {
        let state = loaded();
        let params = ActionParams {
            input: "1".into(),
            mix: 3,
            ..ActionParams::default()
        };
        assert_eq!(
            segment_for(ActionKind::Program, &params, &state, true),
            SegmentState::Unavailable
        );
    }
}
