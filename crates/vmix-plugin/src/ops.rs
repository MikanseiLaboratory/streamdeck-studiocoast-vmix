use urlencoding::encode;
use vmix_pool::{
    acts_preview_name, acts_program_name, normalize_amplitude, tcp_mix_value, Command, VmixState,
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
                || (name.starts_with("Bus")
                    && name.ends_with("Audio")
                    && !name.starts_with("Input"))
        }
        ActionKind::Solo => name == "InputSolo" || name.ends_with("Solo"),
        ActionKind::BusSend => name.starts_with("InputBus") || name == "InputMasterAudio",
        ActionKind::Play => name == "InputPlaying",
        ActionKind::Volume => {
            name.ends_with("Volume")
                || name == "InputAudio"
                || name == "MasterAudio"
                || (name.starts_with("Bus")
                    && name.ends_with("Audio")
                    && !name.starts_with("Input"))
        }
        ActionKind::ReplayJog | ActionKind::ReplaySpeed => name == "ReplayPlaying",
        ActionKind::Headphones => name.ends_with("Headphones"),
        ActionKind::Position => name == "InputPlaying",
        ActionKind::Gain | ActionKind::Mixer | ActionKind::Rate => false,
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
        ActionKind::Program => lit_input(
            state.program.get(&params.mix).copied(),
            state,
            &params.input,
        ),
        ActionKind::Preview => lit_input(
            state.preview.get(&params.mix).copied(),
            state,
            &params.input,
        ),
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
        ActionKind::ReplayJog | ActionKind::ReplaySpeed => flag(state.replay_playing),
        ActionKind::Volume => match audible(state, params) {
            Some(false) => SegmentState::Inactive,
            Some(true) => SegmentState::Active,
            None => SegmentState::Inactive,
        },
        ActionKind::Transition
        | ActionKind::Stinger
        | ActionKind::Title
        | ActionKind::Shortcut
        | ActionKind::Raw
        | ActionKind::Gain
        | ActionKind::Headphones
        | ActionKind::Mixer
        | ActionKind::Rate
        | ActionKind::Position => SegmentState::Neutral,
    }
}

pub fn title_for(kind: ActionKind, params: &ActionParams, state: &VmixState) -> String {
    match kind {
        ActionKind::Volume => format!("{}%", level_to_percent(volume_level(state, params))),
        ActionKind::Headphones => format!("{}%", level_to_percent(headphones_level(state, params))),
        ActionKind::Gain => gain_level(state, params)
            .map(|gain| format!("{gain:.1} dB"))
            .unwrap_or_default(),
        ActionKind::Rate => rate_level(state, params)
            .map(|rate| format!("{rate:.2}x"))
            .unwrap_or_default(),
        ActionKind::ReplaySpeed => replay_speed_level(state, params)
            .map(|speed| format!("{speed:.2}x"))
            .unwrap_or_default(),
        ActionKind::Position => format_timestamp(position_level(state, params).0),
        ActionKind::ReplayJog => {
            if state.replay_playing {
                "▶".into()
            } else {
                "❚❚".into()
            }
        }
        ActionKind::Shortcut => shortcut_name(params),
        ActionKind::Program | ActionKind::Preview | ActionKind::Play | ActionKind::Mute => state
            .resolve_input(&params.input)
            .map(|number| number.to_string())
            .unwrap_or_else(|| params.input.clone()),
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
        ActionKind::Volume
        | ActionKind::ReplayJog
        | ActionKind::Gain
        | ActionKind::Headphones
        | ActionKind::Mixer
        | ActionKind::Rate
        | ActionKind::ReplaySpeed
        | ActionKind::Position => return None,
    };
    Some(Command::Function {
        name: function,
        query: query_for(kind, params),
    })
}

pub fn volume_command(params: &ActionParams, percent: u8) -> Command {
    let value = percent.to_string();
    let (name, query) = match params.audio_target.as_str() {
        "master" => (
            "SetMasterVolume".to_string(),
            Some(format!("Value={value}")),
        ),
        "bus" => (
            format!("SetBus{}Volume", bus_letter(params)),
            Some(format!("Value={value}")),
        ),
        _ => (
            "SetVolume".to_string(),
            Some(join_query(&[
                ("Value", value.as_str()),
                ("Input", params.input.trim()),
            ])),
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

pub fn jog_command(params: &ActionParams, ticks: i32) -> Command {
    let step = if params.step <= 0.0 { 1.0 } else { params.step };
    let frames = (ticks as f32 * step).round() as i32;
    let value = frames.to_string();
    Command::Function {
        name: "ReplayJumpFrames".into(),
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Channel", params.channel.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

pub fn jog_press_command(params: &ActionParams) -> Command {
    Command::Function {
        name: "ReplayPlayPause".into(),
        query: {
            let channel = params.channel.trim();
            if channel.is_empty() {
                None
            } else {
                Some(format!("Channel={channel}"))
            }
        },
    }
}

pub fn dial_level(kind: ActionKind, params: &ActionParams, state: &VmixState) -> Option<f32> {
    match kind {
        ActionKind::Volume => Some(amplitude_to_fader(volume_level(state, params))),
        ActionKind::Headphones => Some(amplitude_to_fader(headphones_level(state, params))),
        ActionKind::Gain => gain_level(state, params).map(|gain| (gain / 24.0).clamp(0.0, 1.0)),
        ActionKind::Rate => {
            rate_level(state, params).map(|rate| (rate / rate_max(params)).clamp(0.0, 1.0))
        }
        ActionKind::ReplaySpeed => {
            replay_speed_level(state, params).map(|speed| (speed / 4.0).clamp(0.0, 1.0))
        }
        ActionKind::Position => {
            let (position, duration) = position_level(state, params);
            if duration == 0 {
                Some(0.0)
            } else {
                Some((position as f32 / duration as f32).clamp(0.0, 1.0))
            }
        }
        ActionKind::ReplayJog => Some(if state.replay_playing { 1.0 } else { 0.0 }),
        ActionKind::Mixer => None,
        _ => None,
    }
}

fn mixer_adjusted_percent(last_fader: Option<f32>, ticks: i32, step: f32) -> u8 {
    adjusted_percent(fader_to_amplitude(last_fader.unwrap_or(0.5)), ticks, step)
}

pub fn rotate_command(
    kind: ActionKind,
    params: &ActionParams,
    state: &VmixState,
    ticks: i32,
) -> Option<Command> {
    match kind {
        ActionKind::Volume => Some(volume_command(
            params,
            adjusted_percent(volume_level(state, params), ticks, params.step),
        )),
        ActionKind::Headphones => Some(headphones_command(
            params,
            adjusted_percent(headphones_level(state, params), ticks, params.step),
        )),
        ActionKind::Mixer => Some(mixer_command(
            params,
            mixer_adjusted_percent(None, ticks, params.step),
        )),
        ActionKind::Gain => Some(gain_command(
            params,
            adjust_range(
                gain_level(state, params).unwrap_or(0.0),
                ticks,
                dial_step(kind, params),
                0.0,
                24.0,
            ),
        )),
        ActionKind::Rate => Some(rate_command(
            params,
            adjust_range(
                rate_level(state, params).unwrap_or(1.0),
                ticks,
                dial_step(kind, params),
                rate_min(params),
                rate_max(params),
            ),
        )),
        ActionKind::ReplaySpeed => Some(replay_speed_command(
            params,
            adjust_range(
                replay_speed_level(state, params).unwrap_or(1.0),
                ticks,
                dial_step(kind, params),
                0.1,
                4.0,
            ),
        )),
        ActionKind::Position => {
            let (position, duration) = position_level(state, params);
            let step = dial_step(kind, params).round() as i64;
            let next = (position as i64 + ticks as i64 * step).max(0) as u64;
            let next = if duration > 0 {
                next.min(duration)
            } else {
                next
            };
            Some(position_command(params, next))
        }
        ActionKind::ReplayJog => Some(jog_command(params, ticks)),
        _ => None,
    }
}

pub fn press_command(kind: ActionKind, params: &ActionParams) -> Option<Command> {
    match kind {
        ActionKind::Volume => Some(mute_command(params)),
        ActionKind::ReplayJog | ActionKind::ReplaySpeed | ActionKind::Position => {
            Some(jog_press_command(params))
        }
        ActionKind::Gain => Some(gain_command(params, 0.0)),
        ActionKind::Rate => Some(rate_command(params, 1.0)),
        ActionKind::Headphones => Some(headphones_command(params, 100)),
        ActionKind::Mixer => Some(mixer_command(params, 100)),
        _ => None,
    }
}

/// vMix UI fader 0–1 from stored amplitude 0–1.
/// Official: Volume = (Amplitude ^ 0.25) * 100.
pub fn amplitude_to_fader(amplitude: f32) -> f32 {
    normalize_amplitude(amplitude).powf(0.25)
}

/// ACTS/XML volume is amplitude. Shortcuts and the dial use the UI fader 0–100.
pub fn level_to_percent(level: f32) -> u8 {
    (amplitude_to_fader(level) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
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

pub fn fader_to_amplitude(fader: f32) -> f32 {
    fader.clamp(0.0, 1.0).powf(4.0)
}

fn dial_step(kind: ActionKind, params: &ActionParams) -> f32 {
    if params.step > 0.0 && (params.step - 1.0).abs() > f32::EPSILON {
        return params.step;
    }
    match kind {
        ActionKind::Rate if is_slow(params) => 0.05,
        ActionKind::Rate | ActionKind::ReplaySpeed => 0.1,
        ActionKind::Position => 1000.0,
        _ => {
            if params.step > 0.0 {
                params.step
            } else {
                1.0
            }
        }
    }
}

fn adjust_range(current: f32, ticks: i32, step: f32, min: f32, max: f32) -> f32 {
    let step = if step <= 0.0 { 1.0 } else { step };
    (current + ticks as f32 * step).clamp(min, max)
}

fn is_slow(params: &ActionParams) -> bool {
    params.rate_mode.eq_ignore_ascii_case("slow")
}

fn rate_min(params: &ActionParams) -> f32 {
    if is_slow(params) {
        0.0
    } else {
        0.1
    }
}

fn rate_max(params: &ActionParams) -> f32 {
    if is_slow(params) {
        1.0
    } else {
        4.0
    }
}

fn headphones_level(state: &VmixState, params: &ActionParams) -> f32 {
    if params.audio_target == "input" && !params.input.trim().is_empty() {
        state
            .resolve_input(&params.input)
            .and_then(|input| state.input_headphones.get(&input).copied())
            .unwrap_or(state.master_headphones)
    } else {
        state.master_headphones
    }
}

fn mixer_channel(params: &ActionParams) -> u8 {
    params
        .index
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|n| (1..=16).contains(n))
        .unwrap_or(1)
}

fn gain_level(state: &VmixState, params: &ActionParams) -> Option<f32> {
    state
        .resolve_input(&params.input)
        .and_then(|input| state.input_gain.get(&input).copied())
}

fn rate_level(state: &VmixState, params: &ActionParams) -> Option<f32> {
    state
        .resolve_input(&params.input)
        .and_then(|input| state.input_rate.get(&input).copied())
}

fn replay_speed_level(state: &VmixState, params: &ActionParams) -> Option<f32> {
    match params.channel.trim().to_ascii_lowercase().as_str() {
        "b" => state.replay_speed_b,
        "a" => state.replay_speed_a.or(state.replay_speed),
        _ => state.replay_speed,
    }
}

fn position_level(state: &VmixState, params: &ActionParams) -> (u64, u64) {
    let Some(input) = state.resolve_input(&params.input) else {
        return (0, 0);
    };
    (
        state.input_position.get(&input).copied().unwrap_or(0),
        state.input_duration.get(&input).copied().unwrap_or(0),
    )
}

fn format_timestamp(ms: u64) -> String {
    let total = ms / 1000;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub fn headphones_command(_params: &ActionParams, percent: u8) -> Command {
    Command::Function {
        name: "SetHeadphonesVolume".into(),
        query: Some(format!("Value={percent}")),
    }
}

pub fn mixer_command(params: &ActionParams, percent: u8) -> Command {
    let value = percent.to_string();
    let name = if params.mixer_mode.eq_ignore_ascii_case("channel") {
        format!("SetVolumeChannelMixer{}", mixer_channel(params))
    } else {
        format!("SetVolumeBusMixer{}", bus_letter(params))
    };
    Command::Function {
        name,
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Input", params.input.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

pub fn gain_command(params: &ActionParams, db: f32) -> Command {
    let value = format_number(db);
    Command::Function {
        name: "SetGain".into(),
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Input", params.input.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

pub fn rate_command(params: &ActionParams, rate: f32) -> Command {
    let value = format_number(rate);
    let name = if is_slow(params) {
        "SetRateSlowMotion"
    } else {
        "SetRate"
    };
    Command::Function {
        name: name.into(),
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Input", params.input.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

pub fn replay_speed_command(params: &ActionParams, speed: f32) -> Command {
    let value = format_number(speed);
    Command::Function {
        name: "ReplayChangeSpeed".into(),
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Channel", params.channel.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

pub fn position_command(params: &ActionParams, ms: u64) -> Command {
    let value = ms.to_string();
    Command::Function {
        name: "SetPosition".into(),
        query: Some(join_query(&[
            ("Value", value.as_str()),
            ("Input", params.input.trim()),
        ]))
        .filter(|query| !query.is_empty()),
    }
}

fn format_number(value: f32) -> String {
    if (value - value.round()).abs() < 0.001 {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
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
        return state
            .bus_solo
            .get(&bus_letter(params))
            .copied()
            .unwrap_or(false);
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
        return state
            .input_master_audio
            .get(&input)
            .copied()
            .unwrap_or(false);
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
        ActionKind::BusSend => {
            join_query(&[("Value", &bus_letter(params).to_string()), ("Input", input)])
        }
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
                parts.push(format!(
                    "SelectedName={}",
                    encode(params.selected_name.trim())
                ));
            }
            parts.join("&")
        }
        ActionKind::Shortcut => shortcut_parts(params)
            .and_then(|(_, query)| query)
            .unwrap_or_default(),
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
    Some((
        line.to_string(),
        if query.is_empty() { None } else { Some(query) },
    ))
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
        let Command::Function { name, query } = command_for(ActionKind::Program, &params).unwrap()
        else {
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
        let Command::Function { query, .. } = command_for(ActionKind::Program, &mix16).unwrap()
        else {
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
        let Command::Function { name, query } = command_for(ActionKind::Shortcut, &params).unwrap()
        else {
            panic!("function");
        };
        assert_eq!(name, "SetText");
        assert_eq!(query.as_deref(), Some("Input=1&Value=hello%20world"));
        let pasted = ActionParams {
            function_name:
                "http://127.0.0.1:8088/api/?Function=SetText&Input=1&Value=hello%20world&Mix=1"
                    .into(),
            input: "9".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = command_for(ActionKind::Shortcut, &pasted).unwrap()
        else {
            panic!("function");
        };
        assert_eq!(name, "SetText");
        assert_eq!(query.as_deref(), Some("Input=1&Value=hello%20world&Mix=1"));
        assert_eq!(shortcut_name(&pasted), "SetText");
    }

    #[test]
    fn volume_round_trips_between_acts_fraction_and_percent() {
        assert_eq!(level_to_percent(0.0625), 50);
        assert_eq!(level_to_percent(6.25), 50);
        assert_eq!(level_to_percent(0.0), 0);
        assert_eq!(level_to_percent(1.0), 100);
        assert_eq!(adjusted_percent(0.0625, 3, 1.0), 53);
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

    #[test]
    fn jog_command_uses_step_and_optional_channel() {
        let params = ActionParams {
            step: 2.0,
            channel: "A".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = jog_command(&params, -3) else {
            panic!("function");
        };
        assert_eq!(name, "ReplayJumpFrames");
        assert_eq!(query.as_deref(), Some("Value=-6&Channel=A"));
        let no_channel = ActionParams {
            step: 1.0,
            ..ActionParams::default()
        };
        let Command::Function { query, .. } = jog_command(&no_channel, 4) else {
            panic!("function");
        };
        assert_eq!(query.as_deref(), Some("Value=4"));
        let Command::Function { name, query } = jog_press_command(&params) else {
            panic!("function");
        };
        assert_eq!(name, "ReplayPlayPause");
        assert_eq!(query.as_deref(), Some("Channel=A"));
    }

    #[test]
    fn volume_and_replay_jog_watch_audio_and_playing_events() {
        assert!(event_affects(ActionKind::Volume, "InputVolume"));
        assert!(event_affects(ActionKind::Volume, "InputAudio"));
        assert!(event_affects(ActionKind::Volume, "MasterAudio"));
        assert!(event_affects(ActionKind::Volume, "BusAAudio"));
        assert!(!event_affects(ActionKind::Volume, "InputBusAAudio"));
        assert!(event_affects(ActionKind::ReplayJog, "ReplayPlaying"));
        assert!(!event_affects(ActionKind::ReplayJog, "Input"));
        assert!(event_affects(ActionKind::Headphones, "MasterHeadphones"));
        assert!(event_affects(ActionKind::Position, "InputPlaying"));
        assert!(!event_affects(ActionKind::Gain, "InputVolume"));
        assert_eq!(
            dial_level(ActionKind::Mixer, &ActionParams::default(), &loaded()),
            None
        );
    }

    #[test]
    fn new_dials_send_official_shortcut_ranges() {
        let input = ActionParams {
            input: "2".into(),
            bus: "M".into(),
            mixer_mode: "bus".into(),
            index: "4".into(),
            channel: "B".into(),
            rate_mode: "slow".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = gain_command(&input, 6.0) else {
            panic!("function");
        };
        assert_eq!(name, "SetGain");
        assert_eq!(query.as_deref(), Some("Value=6&Input=2"));
        let Command::Function { name, query } = headphones_command(&input, 40) else {
            panic!("function");
        };
        assert_eq!(name, "SetHeadphonesVolume");
        assert_eq!(query.as_deref(), Some("Value=40"));
        let Command::Function { name, query } = mixer_command(&input, 80) else {
            panic!("function");
        };
        assert_eq!(name, "SetVolumeBusMixerM");
        assert_eq!(query.as_deref(), Some("Value=80&Input=2"));
        let channel = ActionParams {
            mixer_mode: "channel".into(),
            index: "12".into(),
            input: "3".into(),
            ..ActionParams::default()
        };
        let Command::Function { name, query } = mixer_command(&channel, 25) else {
            panic!("function");
        };
        assert_eq!(name, "SetVolumeChannelMixer12");
        assert_eq!(query.as_deref(), Some("Value=25&Input=3"));
        let Command::Function { name, query } = rate_command(&input, 0.5) else {
            panic!("function");
        };
        assert_eq!(name, "SetRateSlowMotion");
        assert_eq!(query.as_deref(), Some("Value=0.50&Input=2"));
        let Command::Function { name, query } = replay_speed_command(&input, 2.0) else {
            panic!("function");
        };
        assert_eq!(name, "ReplayChangeSpeed");
        assert_eq!(query.as_deref(), Some("Value=2&Channel=B"));
        let Command::Function { name, query } = position_command(&input, 15000) else {
            panic!("function");
        };
        assert_eq!(name, "SetPosition");
        assert_eq!(query.as_deref(), Some("Value=15000&Input=2"));
        assert_eq!(format_timestamp(15000), "00:15");
        assert_eq!(format_timestamp(3_661_000), "1:01:01");
    }
}
