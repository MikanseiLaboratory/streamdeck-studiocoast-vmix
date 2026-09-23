use serde_json::Value;
use streamdeck_plugin::{
    streamdeck_action, ActionContext, ActionPayload, DialRotatePayload, EncoderAction,
    KeypadAction, Result,
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

vmix_key!(ProgramAction, "dev.mikanseilaboratory.vmix.program", ActionKind::Program);
vmix_key!(PreviewAction, "dev.mikanseilaboratory.vmix.preview", ActionKind::Preview);
vmix_key!(TransitionAction, "dev.mikanseilaboratory.vmix.transition", ActionKind::Transition);
vmix_key!(StingerAction, "dev.mikanseilaboratory.vmix.stinger", ActionKind::Stinger);
vmix_key!(FadeToBlackAction, "dev.mikanseilaboratory.vmix.fadetoblack", ActionKind::FadeToBlack);
vmix_key!(OverlayAction, "dev.mikanseilaboratory.vmix.overlay", ActionKind::Overlay);
vmix_key!(RecordingAction, "dev.mikanseilaboratory.vmix.recording", ActionKind::Recording);
vmix_key!(StreamingAction, "dev.mikanseilaboratory.vmix.streaming", ActionKind::Streaming);
vmix_key!(ExternalAction, "dev.mikanseilaboratory.vmix.external", ActionKind::External);
vmix_key!(MultiCorderAction, "dev.mikanseilaboratory.vmix.multicorder", ActionKind::MultiCorder);
vmix_key!(FullscreenAction, "dev.mikanseilaboratory.vmix.fullscreen", ActionKind::Fullscreen);
vmix_key!(ReplayAction, "dev.mikanseilaboratory.vmix.replay", ActionKind::Replay);
vmix_key!(MuteAction, "dev.mikanseilaboratory.vmix.mute", ActionKind::Mute);
vmix_key!(SoloAction, "dev.mikanseilaboratory.vmix.solo", ActionKind::Solo);
vmix_key!(BusSendAction, "dev.mikanseilaboratory.vmix.bussend", ActionKind::BusSend);
vmix_key!(PlayAction, "dev.mikanseilaboratory.vmix.play", ActionKind::Play);
vmix_key!(ListAction, "dev.mikanseilaboratory.vmix.list", ActionKind::List);
vmix_key!(TitleAction, "dev.mikanseilaboratory.vmix.title", ActionKind::Title);
vmix_key!(ShortcutAction, "dev.mikanseilaboratory.vmix.shortcut", ActionKind::Shortcut);
vmix_key!(RawAction, "dev.mikanseilaboratory.vmix.raw", ActionKind::Raw);

#[derive(Default)]
pub struct VolumeDial;

#[streamdeck_action(
    uuid = "dev.mikanseilaboratory.vmix.volume",
    settings = ActionSettings,
    state = AppState
)]
impl EncoderAction for VolumeDial {
    async fn on_will_appear(
        &mut self,
        payload: &ActionPayload,
        ctx: &ActionContext<'_, Self::Settings, Self::State>,
    ) -> Result<()> {
        register(ctx, ActionKind::Volume, payload.is_in_multi_action).await;
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
        register(ctx, ActionKind::Volume, payload.is_in_multi_action).await;
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

    async fn on_property_inspector_did_appear(
        &mut self,
        ctx: &ActionContext<'_, Self::Settings, Self::State>,
    ) -> Result<()> {
        ctx.state().inspector_opened(&ctx.identity().context).await;
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
        std::any::TypeId::of::<RawAction>(),
        std::any::TypeId::of::<VolumeDial>(),
    );
}
