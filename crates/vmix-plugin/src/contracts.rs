use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use streamdeck_plugin::{export_ts, TS};
use vmix_pool::{TargetGroup as PoolGroup, TargetSelector as PoolSelector, VmixInstanceConfig};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub xml_interval_ms: u32,
}

fn default_host() -> String {
    "127.0.0.1".into()
}

fn default_port() -> u16 {
    8099
}

fn default_color() -> String {
    "#4c8dff".into()
}

fn default_true() -> bool {
    true
}

fn default_interval() -> u32 {
    2000
}

impl From<InstanceConfig> for VmixInstanceConfig {
    fn from(value: InstanceConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            host: value.host,
            port: value.port,
            color: value.color,
            enabled: value.enabled,
            xml_interval_ms: u64::from(value.xml_interval_ms),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub members: Vec<String>,
}

impl From<TargetGroup> for PoolGroup {
    fn from(value: TargetGroup) -> Self {
        Self {
            id: value.id,
            name: value.name,
            members: value.members,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase", export)]
pub enum TargetSelector {
    #[default]
    All,
    Group { id: String },
    Instances { ids: Vec<String> },
}

impl From<&TargetSelector> for PoolSelector {
    fn from(value: &TargetSelector) -> Self {
        match value {
            TargetSelector::All => Self::All,
            TargetSelector::Group { id } => Self::Group { id: id.clone() },
            TargetSelector::Instances { ids } => Self::Instances { ids: ids.clone() },
        }
    }
}

pub const LOCALHOST_ID: &str = "localhost";

pub fn localhost_instance() -> InstanceConfig {
    InstanceConfig {
        id: LOCALHOST_ID.into(),
        name: "Localhost".into(),
        host: default_host(),
        port: default_port(),
        color: default_color(),
        enabled: true,
        xml_interval_ms: default_interval(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    #[serde(default)]
    pub instances: Vec<InstanceConfig>,
    #[serde(default)]
    pub groups: Vec<TargetGroup>,
    #[serde(default = "default_fg")]
    pub fg_color: String,
    /// Set after the first launch so an empty list is a user choice, not a missing default.
    #[serde(default)]
    pub seeded: bool,
}

fn default_fg() -> String {
    "#f4f7fb".into()
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            groups: Vec::new(),
            fg_color: default_fg(),
            seeded: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CommonSettings {
    #[serde(default)]
    pub target: TargetSelector,
    #[serde(default = "default_true")]
    pub shared_params: bool,
}

impl Default for CommonSettings {
    fn default() -> Self {
        Self {
            target: TargetSelector::All,
            shared_params: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    #[serde(default)]
    pub long_press: bool,
    #[serde(default)]
    pub long_press_ms: u32,
}

/// Fields used by every action. The inspector shows only the ones that apply.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ActionParams {
    #[serde(default)]
    pub input: String,
    /// `0` is Main. `2`..=`16` are Mix 2 through Mix 16.
    #[serde(default)]
    pub mix: u8,
    #[serde(default = "default_cut")]
    pub effect: String,
    #[serde(default)]
    pub duration_ms: String,
    #[serde(default = "default_one")]
    pub overlay: u8,
    #[serde(default = "default_toggle")]
    pub overlay_mode: String,
    #[serde(default = "default_one")]
    pub stinger: u8,
    #[serde(default = "default_play")]
    pub replay_action: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default = "default_input_target")]
    pub audio_target: String,
    #[serde(default = "default_bus")]
    pub bus: String,
    #[serde(default = "default_next")]
    pub list_action: String,
    #[serde(default)]
    pub index: String,
    #[serde(default = "default_text")]
    pub title_action: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub selected_name: String,
    #[serde(default)]
    pub function_name: String,
    #[serde(default)]
    pub extra: String,
    #[serde(default)]
    pub raw: String,
    #[serde(default = "default_step")]
    pub step: f32,
}

fn default_cut() -> String {
    "Cut".into()
}
fn default_one() -> u8 {
    1
}
fn default_toggle() -> String {
    "toggle".into()
}
fn default_play() -> String {
    "play".into()
}
fn default_input_target() -> String {
    "input".into()
}
fn default_bus() -> String {
    "A".into()
}
fn default_next() -> String {
    "next".into()
}
fn default_text() -> String {
    "settext".into()
}
fn default_step() -> f32 {
    1.0
}

impl Default for ActionParams {
    fn default() -> Self {
        Self {
            input: String::new(),
            mix: 0,
            effect: default_cut(),
            duration_ms: String::new(),
            overlay: 1,
            overlay_mode: default_toggle(),
            stinger: 1,
            replay_action: default_play(),
            channel: String::new(),
            audio_target: default_input_target(),
            bus: default_bus(),
            list_action: default_next(),
            index: String::new(),
            title_action: default_text(),
            value: String::new(),
            selected_name: String::new(),
            function_name: String::new(),
            extra: String::new(),
            raw: String::new(),
            step: default_step(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[ts(export, rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ActionSettings {
    #[serde(default)]
    pub common: CommonSettings,
    #[serde(default)]
    pub advanced: AdvancedSettings,
    #[serde(default)]
    pub shared: ActionParams,
    #[serde(default)]
    pub params: BTreeMap<String, ActionParams>,
}

impl ActionSettings {
    pub fn params_for(&self, instance_id: &str) -> &ActionParams {
        if self.common.shared_params {
            &self.shared
        } else {
            self.params.get(instance_id).unwrap_or(&self.shared)
        }
    }
}

export_ts!(InstanceConfig);
export_ts!(TargetGroup);
export_ts!(TargetSelector);
export_ts!(GlobalSettings);
export_ts!(CommonSettings);
export_ts!(AdvancedSettings);
export_ts!(ActionParams);
export_ts!(ActionSettings);
