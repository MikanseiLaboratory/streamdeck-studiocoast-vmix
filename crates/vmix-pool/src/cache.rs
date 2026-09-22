use std::collections::{HashMap, HashSet};

use vmix_rs::vmix_core::{self, Vmix};
use vmix_rs::vmix_tcp::acts::ActivatorsData;

/// One input row taken from the XML snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedInput {
    pub number: u16,
    pub key: String,
    pub title: String,
    pub short_title: String,
    pub selected_index: Option<String>,
}

/// Activator and XML state for a single vMix instance.
///
/// Volumes are stored as the ACTS 0–1 fraction. XML `volume` (0–100) is divided
/// by 100 on ingest. `input_audio` is true when sound is on, matching ACTS.
#[derive(Clone, Debug, Default)]
pub struct VmixState {
    pub version: String,
    pub edition: String,
    /// User mix (0 = Main, 2..=16) to the program input number.
    pub program: HashMap<u8, u16>,
    pub preview: HashMap<u8, u16>,
    /// Overlay channel 1–8 to the input number while that overlay is on.
    pub overlays: HashMap<u8, u16>,
    pub fade_to_black: bool,
    pub recording: bool,
    pub streaming: bool,
    pub external: bool,
    pub fullscreen: bool,
    pub replay_playing: bool,
    pub multi_corder: bool,
    pub input_audio: HashMap<u16, bool>,
    pub master_audio: bool,
    pub bus_audio: HashMap<char, bool>,
    pub input_solo: HashMap<u16, bool>,
    pub bus_solo: HashMap<char, bool>,
    pub input_bus: HashMap<(u16, char), bool>,
    pub input_master_audio: HashMap<u16, bool>,
    pub input_playing: HashMap<u16, bool>,
    pub input_volume: HashMap<u16, f32>,
    pub master_volume: f32,
    pub bus_volume: HashMap<char, f32>,
    pub inputs: Vec<CachedInput>,
    pub mixes_present: HashSet<u8>,
}

impl VmixState {
    pub fn resolve_input(&self, spec: &str) -> Option<u16> {
        let spec = spec.trim();
        if spec.is_empty() {
            return None;
        }
        if let Ok(number) = spec.parse::<u16>() {
            return Some(number);
        }
        self.inputs
            .iter()
            .find(|input| {
                input.title.eq_ignore_ascii_case(spec)
                    || input.short_title.eq_ignore_ascii_case(spec)
                    || input.key.eq_ignore_ascii_case(spec)
            })
            .map(|input| input.number)
    }

    pub fn mix_available(&self, user_mix: u8) -> bool {
        user_mix == 0 || self.mixes_present.contains(&user_mix)
    }
}

/// Apply one ACTS payload. Returns the activator name used for key filtering.
pub fn apply_acts(state: &mut VmixState, data: ActivatorsData) -> String {
    match data {
        ActivatorsData::Input(input, on) => flag_slot(&mut state.program, 0, input, on, "Input"),
        ActivatorsData::InputMix2(input, on) => flag_slot(&mut state.program, 2, input, on, "InputMix2"),
        ActivatorsData::InputMix3(input, on) => flag_slot(&mut state.program, 3, input, on, "InputMix3"),
        ActivatorsData::InputMix4(input, on) => flag_slot(&mut state.program, 4, input, on, "InputMix4"),
        ActivatorsData::InputMix5(input, on) => flag_slot(&mut state.program, 5, input, on, "InputMix5"),
        ActivatorsData::InputMix6(input, on) => flag_slot(&mut state.program, 6, input, on, "InputMix6"),
        ActivatorsData::InputMix7(input, on) => flag_slot(&mut state.program, 7, input, on, "InputMix7"),
        ActivatorsData::InputMix8(input, on) => flag_slot(&mut state.program, 8, input, on, "InputMix8"),
        ActivatorsData::InputMix9(input, on) => flag_slot(&mut state.program, 9, input, on, "InputMix9"),
        ActivatorsData::InputMix10(input, on) => flag_slot(&mut state.program, 10, input, on, "InputMix10"),
        ActivatorsData::InputMix11(input, on) => flag_slot(&mut state.program, 11, input, on, "InputMix11"),
        ActivatorsData::InputMix12(input, on) => flag_slot(&mut state.program, 12, input, on, "InputMix12"),
        ActivatorsData::InputMix13(input, on) => flag_slot(&mut state.program, 13, input, on, "InputMix13"),
        ActivatorsData::InputMix14(input, on) => flag_slot(&mut state.program, 14, input, on, "InputMix14"),
        ActivatorsData::InputMix15(input, on) => flag_slot(&mut state.program, 15, input, on, "InputMix15"),
        ActivatorsData::InputMix16(input, on) => flag_slot(&mut state.program, 16, input, on, "InputMix16"),
        ActivatorsData::InputPreview(input, on) => {
            flag_slot(&mut state.preview, 0, input, on, "InputPreview")
        }
        ActivatorsData::InputPreviewMix2(input, on) => {
            flag_slot(&mut state.preview, 2, input, on, "InputPreviewMix2")
        }
        ActivatorsData::InputPreviewMix3(input, on) => {
            flag_slot(&mut state.preview, 3, input, on, "InputPreviewMix3")
        }
        ActivatorsData::InputPreviewMix4(input, on) => {
            flag_slot(&mut state.preview, 4, input, on, "InputPreviewMix4")
        }
        ActivatorsData::InputPreviewMix5(input, on) => {
            flag_slot(&mut state.preview, 5, input, on, "InputPreviewMix5")
        }
        ActivatorsData::InputPreviewMix6(input, on) => {
            flag_slot(&mut state.preview, 6, input, on, "InputPreviewMix6")
        }
        ActivatorsData::InputPreviewMix7(input, on) => {
            flag_slot(&mut state.preview, 7, input, on, "InputPreviewMix7")
        }
        ActivatorsData::InputPreviewMix8(input, on) => {
            flag_slot(&mut state.preview, 8, input, on, "InputPreviewMix8")
        }
        ActivatorsData::InputPreviewMix9(input, on) => {
            flag_slot(&mut state.preview, 9, input, on, "InputPreviewMix9")
        }
        ActivatorsData::InputPreviewMix10(input, on) => {
            flag_slot(&mut state.preview, 10, input, on, "InputPreviewMix10")
        }
        ActivatorsData::InputPreviewMix11(input, on) => {
            flag_slot(&mut state.preview, 11, input, on, "InputPreviewMix11")
        }
        ActivatorsData::InputPreviewMix12(input, on) => {
            flag_slot(&mut state.preview, 12, input, on, "InputPreviewMix12")
        }
        ActivatorsData::InputPreviewMix13(input, on) => {
            flag_slot(&mut state.preview, 13, input, on, "InputPreviewMix13")
        }
        ActivatorsData::InputPreviewMix14(input, on) => {
            flag_slot(&mut state.preview, 14, input, on, "InputPreviewMix14")
        }
        ActivatorsData::InputPreviewMix15(input, on) => {
            flag_slot(&mut state.preview, 15, input, on, "InputPreviewMix15")
        }
        ActivatorsData::InputPreviewMix16(input, on) => {
            flag_slot(&mut state.preview, 16, input, on, "InputPreviewMix16")
        }
        ActivatorsData::Overlay1(input, on) => flag_slot(&mut state.overlays, 1, input, on, "Overlay1"),
        ActivatorsData::Overlay2(input, on) => flag_slot(&mut state.overlays, 2, input, on, "Overlay2"),
        ActivatorsData::Overlay3(input, on) => flag_slot(&mut state.overlays, 3, input, on, "Overlay3"),
        ActivatorsData::Overlay4(input, on) => flag_slot(&mut state.overlays, 4, input, on, "Overlay4"),
        ActivatorsData::Overlay5(input, on) => flag_slot(&mut state.overlays, 5, input, on, "Overlay5"),
        ActivatorsData::Overlay6(input, on) => flag_slot(&mut state.overlays, 6, input, on, "Overlay6"),
        ActivatorsData::Overlay7(input, on) => flag_slot(&mut state.overlays, 7, input, on, "Overlay7"),
        ActivatorsData::Overlay8(input, on) => flag_slot(&mut state.overlays, 8, input, on, "Overlay8"),
        ActivatorsData::InputPlaying(input, on) => {
            state.input_playing.insert(input, on);
            "InputPlaying".into()
        }
        ActivatorsData::InputAudio(input, on) => {
            state.input_audio.insert(input, on);
            "InputAudio".into()
        }
        ActivatorsData::InputSolo(input, on) => {
            state.input_solo.insert(input, on);
            "InputSolo".into()
        }
        ActivatorsData::InputBusAAudio(input, on) => bus_input(state, input, 'A', on),
        ActivatorsData::InputBusBAudio(input, on) => bus_input(state, input, 'B', on),
        ActivatorsData::InputBusCAudio(input, on) => bus_input(state, input, 'C', on),
        ActivatorsData::InputBusDAudio(input, on) => bus_input(state, input, 'D', on),
        ActivatorsData::InputBusEAudio(input, on) => bus_input(state, input, 'E', on),
        ActivatorsData::InputBusFAudio(input, on) => bus_input(state, input, 'F', on),
        ActivatorsData::InputBusGAudio(input, on) => bus_input(state, input, 'G', on),
        ActivatorsData::InputMasterAudio(input, on) => {
            state.input_master_audio.insert(input, on);
            "InputMasterAudio".into()
        }
        ActivatorsData::MasterAudio(on) => {
            state.master_audio = on;
            "MasterAudio".into()
        }
        ActivatorsData::BusAAudio(on) => bus_flag(state, 'A', on),
        ActivatorsData::BusBAudio(on) => bus_flag(state, 'B', on),
        ActivatorsData::BusCAudio(on) => bus_flag(state, 'C', on),
        ActivatorsData::BusDAudio(on) => bus_flag(state, 'D', on),
        ActivatorsData::BusEAudio(on) => bus_flag(state, 'E', on),
        ActivatorsData::BusFAudio(on) => bus_flag(state, 'F', on),
        ActivatorsData::BusGAudio(on) => bus_flag(state, 'G', on),
        ActivatorsData::BusASolo(on) => bus_solo(state, 'A', on),
        ActivatorsData::BusBSolo(on) => bus_solo(state, 'B', on),
        ActivatorsData::BusCSolo(on) => bus_solo(state, 'C', on),
        ActivatorsData::BusDSolo(on) => bus_solo(state, 'D', on),
        ActivatorsData::BusESolo(on) => bus_solo(state, 'E', on),
        ActivatorsData::BusFSolo(on) => bus_solo(state, 'F', on),
        ActivatorsData::BusGSolo(on) => bus_solo(state, 'G', on),
        ActivatorsData::FadeToBlack(on) => {
            state.fade_to_black = on;
            "FadeToBlack".into()
        }
        ActivatorsData::Recording(on) => {
            state.recording = on;
            "Recording".into()
        }
        ActivatorsData::Streaming(on) => {
            state.streaming = on;
            "Streaming".into()
        }
        ActivatorsData::External(on) => {
            state.external = on;
            "External".into()
        }
        ActivatorsData::Fullscreen(on) => {
            state.fullscreen = on;
            "Fullscreen".into()
        }
        ActivatorsData::ReplayPlaying(on) => {
            state.replay_playing = on;
            "ReplayPlaying".into()
        }
        ActivatorsData::InputVolume(input, level) => {
            state.input_volume.insert(input, level);
            "InputVolume".into()
        }
        ActivatorsData::MasterVolume(level) => {
            state.master_volume = level;
            "MasterVolume".into()
        }
        ActivatorsData::BusAVolume(level) => bus_volume(state, 'A', level),
        ActivatorsData::BusBVolume(level) => bus_volume(state, 'B', level),
        ActivatorsData::BusCVolume(level) => bus_volume(state, 'C', level),
        ActivatorsData::BusDVolume(level) => bus_volume(state, 'D', level),
        ActivatorsData::BusEVolume(level) => bus_volume(state, 'E', level),
        ActivatorsData::BusFVolume(level) => bus_volume(state, 'F', level),
        ActivatorsData::BusGVolume(level) => bus_volume(state, 'G', level),
        ActivatorsData::InputHeadphones(_, _) => "InputHeadphones".into(),
        ActivatorsData::MasterHeadphones(_) => "MasterHeadphones".into(),
        ActivatorsData::Unknown(parts) => parts.first().cloned().unwrap_or_else(|| "Unknown".into()),
    }
}

fn flag_slot(
    map: &mut HashMap<u8, u16>,
    slot: u8,
    input: u16,
    active: bool,
    name: &str,
) -> String {
    if active {
        map.insert(slot, input);
    } else if map.get(&slot) == Some(&input) {
        map.remove(&slot);
    }
    name.to_string()
}

fn bus_input(state: &mut VmixState, input: u16, bus: char, on: bool) -> String {
    state.input_bus.insert((input, bus), on);
    format!("InputBus{bus}Audio")
}

fn bus_flag(state: &mut VmixState, bus: char, on: bool) -> String {
    state.bus_audio.insert(bus, on);
    format!("Bus{bus}Audio")
}

fn bus_solo(state: &mut VmixState, bus: char, on: bool) -> String {
    state.bus_solo.insert(bus, on);
    format!("Bus{bus}Solo")
}

fn bus_volume(state: &mut VmixState, bus: char, level: f32) -> String {
    state.bus_volume.insert(bus, level);
    format!("Bus{bus}Volume")
}

pub fn apply_xml(state: &mut VmixState, xml: &str) -> Result<(), vmix_core::quick_xml::DeError> {
    let parsed = vmix_core::from_str(xml)?;
    merge_xml(state, &parsed);
    Ok(())
}

fn merge_xml(state: &mut VmixState, vmix: &Vmix) {
    if !vmix.version.is_empty() {
        state.version = vmix.version.clone();
    }
    if !vmix.edition.is_empty() {
        state.edition = vmix.edition.clone();
    }
    state.fade_to_black = vmix.fade_to_black;
    state.recording = vmix.recording;
    state.streaming = vmix.streaming;
    state.external = vmix.external;
    state.fullscreen = vmix.fullscreen;
    state.multi_corder = vmix.multi_corder;
    state.mixes_present.clear();
    state.mixes_present.insert(0);
    if let Ok(input) = vmix.active.parse::<u16>() {
        if input > 0 {
            state.program.insert(0, input);
        }
    }
    if let Ok(input) = vmix.preview.parse::<u16>() {
        if input > 0 {
            state.preview.insert(0, input);
        }
    }
    for (index, mix) in vmix.mix.iter().enumerate() {
        let user = match mix.number.parse::<u8>() {
            Ok(number) if (2..=16).contains(&number) => number,
            Ok(0) | Ok(1) => 0,
            _ if index == 0 => 0,
            _ => (index as u8).saturating_add(1),
        };
        state.mixes_present.insert(user);
        if let Ok(input) = mix.active.parse::<u16>() {
            if input > 0 {
                state.program.insert(user, input);
            }
        }
        if let Ok(input) = mix.preview.parse::<u16>() {
            if input > 0 {
                state.preview.insert(user, input);
            }
        }
    }
    state.inputs.clear();
    for input in &vmix.inputs.input {
        let Ok(number) = input.number.parse::<u16>() else {
            continue;
        };
        if let Some(muted) = input.muted {
            state.input_audio.insert(number, !muted);
        }
        if let Some(volume) = input.volume {
            state.input_volume.insert(number, (volume as f32 / 100.0).clamp(0.0, 1.0));
        }
        if let Some(solo) = input.solo {
            state.input_solo.insert(number, solo);
        }
        state.inputs.push(CachedInput {
            number,
            key: input.key.clone(),
            title: input.title.clone(),
            short_title: input.short_title.clone(),
            selected_index: input.selected_index.clone(),
        });
    }
    state.master_volume = (vmix.audio.master.volume as f32 / 100.0).clamp(0.0, 1.0);
    state.master_audio = !vmix.audio.master.muted;
    apply_bus(state, 'A', vmix.audio.bus_a.as_ref());
    apply_bus(state, 'B', vmix.audio.bus_b.as_ref());
    apply_bus(state, 'C', vmix.audio.bus_c.as_ref());
    apply_bus(state, 'D', vmix.audio.bus_d.as_ref());
    apply_bus(state, 'E', vmix.audio.bus_e.as_ref());
    apply_bus(state, 'F', vmix.audio.bus_f.as_ref());
    apply_bus(state, 'G', vmix.audio.bus_g.as_ref());
    for overlay in &vmix.overlays.overlay {
        let Ok(channel) = overlay.number.parse::<u8>() else {
            continue;
        };
        match overlay.input.as_deref().and_then(|value| value.parse::<u16>().ok()) {
            Some(input) if input > 0 => {
                state.overlays.insert(channel, input);
            }
            _ => {
                state.overlays.remove(&channel);
            }
        }
    }
}

fn apply_bus(state: &mut VmixState, bus: char, audio: Option<&vmix_core::AudioBus>) {
    let Some(audio) = audio else {
        return;
    };
    state.bus_audio.insert(bus, !audio.muted);
    state.bus_volume.insert(bus, (audio.volume as f32 / 100.0).clamp(0.0, 1.0));
    if let Some(solo) = audio.solo {
        state.bus_solo.insert(bus, solo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_snapshot_keeps_main_and_numbered_mixes() {
        let mut state = VmixState::default();
        apply_xml(&mut state, crate::mock::sample_xml()).unwrap();
        assert_eq!(state.program.get(&0), Some(&1));
        assert_eq!(state.preview.get(&0), Some(&2));
        assert!(state.mixes_present.contains(&2));
        assert_eq!(state.program.get(&2), Some(&2));
        assert_eq!(state.resolve_input("Guest"), Some(2));
        assert!(!state.multi_corder);
        assert!((state.input_volume.get(&1).copied().unwrap_or(0.0) - 0.8).abs() < 0.001);
    }
}
