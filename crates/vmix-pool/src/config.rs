use serde::{Deserialize, Serialize};

/// One named vMix instance. `id` stays stable so keys can refer to it after a reorder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmixInstanceConfig {
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
    /// How often to refresh the XML snapshot, in milliseconds. `0` disables polling.
    #[serde(default = "default_interval")]
    pub xml_interval_ms: u64,
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

fn default_interval() -> u64 {
    2000
}

impl VmixInstanceConfig {
    /// Connection identity. Name and color changes do not force a reconnect.
    pub fn endpoint_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.host.trim(),
            self.port,
            self.enabled,
            self.xml_interval_ms
        )
    }
}

/// A named set of instance ids used as an action target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub members: Vec<String>,
}

/// Which instances an action should talk to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TargetSelector {
    #[default]
    All,
    Group { id: String },
    Instances { ids: Vec<String> },
}

/// Resolve a selector to existing instance ids, preserving configuration order.
///
/// `All` includes only enabled instances. An explicit group or id list keeps
/// disabled instances so the key can show them as unavailable.
pub fn resolve_targets(
    selector: &TargetSelector,
    instances: &[VmixInstanceConfig],
    groups: &[TargetGroup],
) -> Vec<String> {
    match selector {
        TargetSelector::All => instances
            .iter()
            .filter(|instance| instance.enabled)
            .map(|instance| instance.id.clone())
            .collect(),
        TargetSelector::Group { id } => {
            let Some(group) = groups.iter().find(|group| group.id == *id) else {
                return Vec::new();
            };
            instances
                .iter()
                .filter(|instance| group.members.iter().any(|member| member == &instance.id))
                .map(|instance| instance.id.clone())
                .collect()
        }
        TargetSelector::Instances { ids } => instances
            .iter()
            .filter(|instance| ids.iter().any(|id| id == &instance.id))
            .map(|instance| instance.id.clone())
            .collect(),
    }
}
