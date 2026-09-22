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
    Shortcut,
    Raw,
    Volume,
}

impl ActionKind {
    pub fn has_toggle_state(self) -> bool {
        !matches!(
            self,
            Self::Transition | Self::Stinger | Self::Title | Self::Shortcut | Self::Raw | Self::Volume
        )
    }

    pub fn confirms_press(self) -> bool {
        matches!(
            self,
            Self::Transition | Self::Stinger | Self::Title | Self::Shortcut | Self::Raw
        )
    }
}
