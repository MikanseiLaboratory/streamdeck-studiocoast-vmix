#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Program,
    Preview,
    Transition,
    Stinger,
    FadeToBlack,
    Overlay,
    Recording,
    Streaming,
    External,
    MultiCorder,
    Fullscreen,
    Replay,
    Mute,
    Solo,
    BusSend,
    Play,
    List,
    Title,
    Loop,
    Playlist,
    Restart,
    QuickPlay,
    Snapshot,
    Go,
    Countdown,
    Browser,
    Script,
    Preset,
    PreviewStep,
    Ptz,
    ReplayCamera,
    AudioAuto,
    SoloAllOff,
    DataSource,
    Shortcut,
    Raw,
    RawDial,
    Volume,
    ReplayJog,
    Gain,
    Headphones,
    Mixer,
    Rate,
    ReplaySpeed,
    Position,
}

impl ActionKind {
    pub fn has_toggle_state(self) -> bool {
        !matches!(
            self,
            Self::Transition
                | Self::Stinger
                | Self::Title
                | Self::Restart
                | Self::QuickPlay
                | Self::Snapshot
                | Self::Go
                | Self::Countdown
                | Self::Browser
                | Self::Script
                | Self::Preset
                | Self::PreviewStep
                | Self::Ptz
                | Self::ReplayCamera
                | Self::AudioAuto
                | Self::SoloAllOff
                | Self::DataSource
                | Self::Shortcut
                | Self::Raw
                | Self::RawDial
                | Self::Volume
                | Self::ReplayJog
                | Self::Gain
                | Self::Headphones
                | Self::Mixer
                | Self::Rate
                | Self::ReplaySpeed
                | Self::Position
        )
    }

    pub fn confirms_press(self) -> bool {
        matches!(
            self,
            Self::Transition
                | Self::Stinger
                | Self::Title
                | Self::Restart
                | Self::QuickPlay
                | Self::Snapshot
                | Self::Go
                | Self::Countdown
                | Self::Browser
                | Self::Script
                | Self::Preset
                | Self::PreviewStep
                | Self::Ptz
                | Self::ReplayCamera
                | Self::AudioAuto
                | Self::SoloAllOff
                | Self::DataSource
                | Self::Shortcut
                | Self::Raw
        )
    }

    pub fn is_dial(self) -> bool {
        matches!(
            self,
            Self::RawDial
                | Self::Volume
                | Self::ReplayJog
                | Self::Gain
                | Self::Headphones
                | Self::Mixer
                | Self::Rate
                | Self::ReplaySpeed
                | Self::Position
        )
    }
}
