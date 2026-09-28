use serde_json::Value;
use streamdeck_plugin::{
    streamdeck_action, ActionContext, ActionPayload, DialRotatePayload, EncoderAction,
    KeypadAction, Result, TriggerDescription,
};

use crate::contracts::ActionSettings;
use crate::kind::ActionKind;
use crate::state::AppState;

macro_rules! vmix_key {
    ($name:ident, $uuid:literal, $kind:expr) => {
        #[derive(Default)]
        pub struct $name;

        #[streamdeck_action(uuid = $uuid, settings = ActionSettings, state = AppState)]
        impl KeypadAction for $name {
            async fn on_will_appear(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_will_disappear(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().remove_key(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_did_receive_settings(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                Ok(())
            }

            async fn on_key_down(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().press(&ctx.identity().context);
                Ok(())
            }

            async fn on_property_inspector_did_appear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().inspector_opened(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_property_inspector_did_disappear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().inspector_closed(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_property_inspector_message(
                &mut self,
                payload: &Value,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .handle_inspector(&ctx.identity().context, payload.clone());
                Ok(())
            }
        }
    };
}

async fn register(
    ctx: &ActionContext<'_, ActionSettings, AppState>,
    kind: ActionKind,
    multi: bool,
) {
    ctx.state().note_sender(ctx.sender().clone()).await;
    ctx.state()
        .upsert_key(
            &ctx.identity().context,
            kind,
            &ctx.identity().action_id,
            ctx.settings(),
            multi,
        )
        .await;
}

vmix_key!(
    ProgramAction,
    "dev.mikanseilaboratory.vmix.program",
    ActionKind::Program
);
vmix_key!(
    PreviewAction,
    "dev.mikanseilaboratory.vmix.preview",
    ActionKind::Preview
);
vmix_key!(
    TransitionAction,
    "dev.mikanseilaboratory.vmix.transition",
    ActionKind::Transition
);
vmix_key!(
    StingerAction,
    "dev.mikanseilaboratory.vmix.stinger",
    ActionKind::Stinger
);
vmix_key!(
    FadeToBlackAction,
    "dev.mikanseilaboratory.vmix.fadetoblack",
    ActionKind::FadeToBlack
);
vmix_key!(
    OverlayAction,
    "dev.mikanseilaboratory.vmix.overlay",
    ActionKind::Overlay
);
vmix_key!(
    RecordingAction,
    "dev.mikanseilaboratory.vmix.recording",
    ActionKind::Recording
);
vmix_key!(
    StreamingAction,
    "dev.mikanseilaboratory.vmix.streaming",
    ActionKind::Streaming
);
vmix_key!(
    ExternalAction,
    "dev.mikanseilaboratory.vmix.external",
    ActionKind::External
);
vmix_key!(
    MultiCorderAction,
    "dev.mikanseilaboratory.vmix.multicorder",
    ActionKind::MultiCorder
);
vmix_key!(
    FullscreenAction,
    "dev.mikanseilaboratory.vmix.fullscreen",
    ActionKind::Fullscreen
);
vmix_key!(
    ReplayAction,
    "dev.mikanseilaboratory.vmix.replay",
    ActionKind::Replay
);
vmix_key!(
    MuteAction,
    "dev.mikanseilaboratory.vmix.mute",
    ActionKind::Mute
);
vmix_key!(
    SoloAction,
    "dev.mikanseilaboratory.vmix.solo",
    ActionKind::Solo
);
vmix_key!(
    BusSendAction,
    "dev.mikanseilaboratory.vmix.bussend",
    ActionKind::BusSend
);
vmix_key!(
    PlayAction,
    "dev.mikanseilaboratory.vmix.play",
    ActionKind::Play
);
vmix_key!(
    ListAction,
    "dev.mikanseilaboratory.vmix.list",
    ActionKind::List
);
vmix_key!(
    TitleAction,
    "dev.mikanseilaboratory.vmix.title",
    ActionKind::Title
);
vmix_key!(
    ShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut",
    ActionKind::Shortcut
);
vmix_key!(
    GeneralShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-general",
    ActionKind::Shortcut
);
vmix_key!(
    AudioShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-audio",
    ActionKind::Shortcut
);
vmix_key!(
    TransitionShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-transition",
    ActionKind::Shortcut
);
vmix_key!(
    OutputShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-output",
    ActionKind::Shortcut
);
vmix_key!(
    TitleShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-title",
    ActionKind::Shortcut
);
vmix_key!(
    InputShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-input",
    ActionKind::Shortcut
);
vmix_key!(
    OverlayShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-overlay",
    ActionKind::Shortcut
);
vmix_key!(
    PlaylistShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-playlist",
    ActionKind::Shortcut
);
vmix_key!(
    ScriptingShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-scripting",
    ActionKind::Shortcut
);
vmix_key!(
    ReplayShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-replay",
    ActionKind::Shortcut
);
vmix_key!(
    NdiShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-ndi",
    ActionKind::Shortcut
);
vmix_key!(
    OmtShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-omt",
    ActionKind::Shortcut
);
vmix_key!(
    PtzShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-ptz",
    ActionKind::Shortcut
);
vmix_key!(
    PresetShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-preset",
    ActionKind::Shortcut
);
vmix_key!(
    DataSourcesShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-datasources",
    ActionKind::Shortcut
);
vmix_key!(
    BrowserShortcutAction,
    "dev.mikanseilaboratory.vmix.shortcut-browser",
    ActionKind::Shortcut
);
vmix_key!(
    RawAction,
    "dev.mikanseilaboratory.vmix.raw",
    ActionKind::Raw
);

macro_rules! vmix_dial {
    ($name:ident, $uuid:literal, $kind:expr) => {
        #[derive(Default)]
        pub struct $name;

        #[streamdeck_action(uuid = $uuid, settings = ActionSettings, state = AppState)]
        impl EncoderAction for $name {
            async fn on_will_appear(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                let _ = ctx.set_trigger_description(trigger_for($kind));
                Ok(())
            }

            async fn on_will_disappear(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().remove_key(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_did_receive_settings(
                &mut self,
                payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                register(ctx, $kind, payload.is_in_multi_action).await;
                let _ = ctx.set_trigger_description(trigger_for($kind));
                Ok(())
            }

            async fn on_dial_rotate(
                &mut self,
                payload: &DialRotatePayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().rotate(&ctx.identity().context, payload.ticks);
                Ok(())
            }

            async fn on_dial_down(
                &mut self,
                _payload: &ActionPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().dial_down(&ctx.identity().context);
                Ok(())
            }

            async fn on_touch_tap(
                &mut self,
                _payload: &streamdeck_plugin::TouchTapPayload,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().touch_tap(&ctx.identity().context);
                Ok(())
            }

            async fn on_property_inspector_did_appear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().inspector_opened(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_property_inspector_did_disappear(
                &mut self,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state().inspector_closed(&ctx.identity().context).await;
                Ok(())
            }

            async fn on_property_inspector_message(
                &mut self,
                payload: &Value,
                ctx: &ActionContext<'_, Self::Settings, Self::State>,
            ) -> Result<()> {
                ctx.state()
                    .handle_inspector(&ctx.identity().context, payload.clone());
                Ok(())
            }
        }
    };
}

fn trigger_for(kind: ActionKind) -> TriggerDescription {
    match kind {
        ActionKind::Volume => TriggerDescription {
            rotate: Some("Volume".into()),
            push: Some("Mute".into()),
            touch: Some("Mute".into()),
            long_touch: None,
        },
        ActionKind::ReplayJog => TriggerDescription {
            rotate: Some("Jog frames".into()),
            push: Some("Play / Pause".into()),
            touch: Some("Play / Pause".into()),
            long_touch: None,
        },
        _ => TriggerDescription::default(),
    }
}

vmix_dial!(
    VolumeDial,
    "dev.mikanseilaboratory.vmix.volume",
    ActionKind::Volume
);
vmix_dial!(
    ReplayJogDial,
    "dev.mikanseilaboratory.vmix.replayjog",
    ActionKind::ReplayJog
);

pub fn force_link() {
    let _ = (
        std::any::TypeId::of::<ProgramAction>(),
        std::any::TypeId::of::<PreviewAction>(),
        std::any::TypeId::of::<TransitionAction>(),
        std::any::TypeId::of::<StingerAction>(),
        std::any::TypeId::of::<FadeToBlackAction>(),
        std::any::TypeId::of::<OverlayAction>(),
        std::any::TypeId::of::<RecordingAction>(),
        std::any::TypeId::of::<StreamingAction>(),
        std::any::TypeId::of::<ExternalAction>(),
        std::any::TypeId::of::<MultiCorderAction>(),
        std::any::TypeId::of::<FullscreenAction>(),
        std::any::TypeId::of::<ReplayAction>(),
        std::any::TypeId::of::<MuteAction>(),
        std::any::TypeId::of::<SoloAction>(),
        std::any::TypeId::of::<BusSendAction>(),
        std::any::TypeId::of::<PlayAction>(),
        std::any::TypeId::of::<ListAction>(),
        std::any::TypeId::of::<TitleAction>(),
        std::any::TypeId::of::<ShortcutAction>(),
        std::any::TypeId::of::<GeneralShortcutAction>(),
        std::any::TypeId::of::<AudioShortcutAction>(),
        std::any::TypeId::of::<TransitionShortcutAction>(),
        std::any::TypeId::of::<OutputShortcutAction>(),
        std::any::TypeId::of::<TitleShortcutAction>(),
        std::any::TypeId::of::<InputShortcutAction>(),
        std::any::TypeId::of::<OverlayShortcutAction>(),
        std::any::TypeId::of::<PlaylistShortcutAction>(),
        std::any::TypeId::of::<ScriptingShortcutAction>(),
        std::any::TypeId::of::<ReplayShortcutAction>(),
        std::any::TypeId::of::<NdiShortcutAction>(),
        std::any::TypeId::of::<OmtShortcutAction>(),
        std::any::TypeId::of::<PtzShortcutAction>(),
        std::any::TypeId::of::<PresetShortcutAction>(),
        std::any::TypeId::of::<DataSourcesShortcutAction>(),
        std::any::TypeId::of::<BrowserShortcutAction>(),
        std::any::TypeId::of::<RawAction>(),
        std::any::TypeId::of::<VolumeDial>(),
        std::any::TypeId::of::<ReplayJogDial>(),
    );
}
