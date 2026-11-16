//! Errors surfaced by the device layer.

use thiserror::Error;

/// Something that went wrong talking to a device.
#[derive(Debug, Error)]
pub enum DeviceError {
    /// No device with the given id is currently tracked.
    #[error("device not found: {0}")]
    NotFound(String),

    /// A matching device is present but could not be opened for access
    /// (on Linux this usually means missing udev rules).
    #[error("permission denied opening device (see udev setup)")]
    PermissionDenied,

    /// The setting id is not known for this device model.
    #[error("unknown setting: {0}")]
    UnknownSetting(String),

    /// The value is not valid for this setting.
    #[error("invalid value {value:?} for setting {setting:?}")]
    UnknownValue { setting: String, value: String },

    /// The setting addresses one specific transmitter (see `V2Target::Tx`,
    /// e.g. Voice Tone) and needs `DeviceManager::set_tx`, not `set`.
    #[error("setting {0:?} must be targeted at a specific transmitter (use set_tx)")]
    RequiresTx(String),

    /// The setting doesn't address a specific transmitter and needs
    /// `DeviceManager::set`, not `set_tx`.
    #[error("setting {0:?} does not target a specific transmitter (use set)")]
    UnexpectedTx(String),

    /// A USB transfer failed.
    #[error("usb error: {0}")]
    Usb(String),

    /// The device task is no longer running.
    #[error("device disconnected")]
    Disconnected,
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, DeviceError>;
