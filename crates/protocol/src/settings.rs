//! Data-driven description of a device's controllable settings.
//!
//! A [`Setting`] is pure metadata: an id, a label, the command it maps to, and
//! the discrete options it accepts. The CLI enumerates these to build its
//! `get`/`set` surface and the GUI renders one control per setting, so adding a
//! setting to a [`DeviceModel`](crate::models::DeviceModel) automatically
//! extends both front-ends with no other changes.

use serde::Serialize;

/// How a setting should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettingKind {
    /// Two options — rendered as a switch.
    Toggle,
    /// More than two mutually exclusive options — rendered as a segmented picker.
    Enum,
}

/// Where a v2 command is addressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum V2Target {
    /// Fixed for every instance of the setting: the receiver (`0x0000`) or a
    /// broadcast to every connected transmitter (`0xffff`).
    Fixed(u16),
    /// One specific transmitter (unit `1` or `2`), chosen by the caller when
    /// building the command — for settings that don't mirror across TX (see
    /// Voice Tone in `PROTOCOL.md`).
    Tx,
}

/// One selectable value of a [`Setting`].
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SettingOption {
    /// Stable slug used on the CLI and wire-agnostic APIs, e.g. `"strong"`.
    pub value: &'static str,
    /// Human label, e.g. `"Strong"`.
    pub label: &'static str,
    /// The single payload byte sent for this option.
    pub wire: u8,
}

/// A controllable setting on a device.
#[derive(Debug, Clone, Serialize)]
pub struct Setting {
    /// Stable slug, e.g. `"noise-cancel"`.
    pub id: &'static str,
    /// Human label, e.g. `"Noise Cancel"`.
    pub label: &'static str,
    /// Group heading used by the compact UI to cluster related settings.
    pub group: &'static str,
    /// Presentation hint.
    pub kind: SettingKind,
    /// 16-bit command id used to write this setting on v1 firmware's 19-byte
    /// command shape. `None` if the setting doesn't exist there (it was
    /// introduced by the v2 firmware update).
    pub v1_command: Option<u16>,
    /// Command id used to write this setting inside v2 firmware's
    /// target-addressed 22-byte command shape. Every setting is reachable
    /// this way.
    pub v2_command: u16,
    /// Target for [`v2_command`](Self::v2_command).
    pub v2_target: V2Target,
    /// Allowed options, in display order.
    pub options: &'static [SettingOption],
    /// Ids of settings this one is mutually exclusive with: when any of them is
    /// active ("on"), this setting cannot be enabled and should be locked.
    pub exclusive_with: &'static [&'static str],
    /// Optional caveat shown next to the control (e.g. reboot warning).
    pub note: Option<&'static str>,
}

impl Setting {
    /// Look up an option by its slug.
    pub fn option(&self, value: &str) -> Option<&SettingOption> {
        self.options.iter().find(|o| o.value == value)
    }

    /// Look up the option whose wire byte matches `wire`.
    pub fn option_for_wire(&self, wire: u8) -> Option<&SettingOption> {
        self.options.iter().find(|o| o.wire == wire)
    }

    /// The valid option slugs, for error messages and CLI help.
    pub fn value_slugs(&self) -> Vec<&'static str> {
        self.options.iter().map(|o| o.value).collect()
    }
}
