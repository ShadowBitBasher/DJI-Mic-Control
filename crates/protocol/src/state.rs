//! Decoded device state shared by every front-end.
//!
//! [`DeviceStatus`] is the model-agnostic snapshot a device reports. Each
//! [`DeviceModel`](crate::models::DeviceModel) fills it in from its own
//! heartbeat format; the CLI and GUI only ever see this struct.

use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

/// A four-component firmware version, e.g. `01.01.00.56`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Firmware(pub [u8; 4]);

impl std::fmt::Display for Firmware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a:02}.{b:02}.{c:02}.{d:02}")
    }
}

// Serialize as the dotted string so front-ends display "01.01.00.56", not an array.
impl Serialize for Firmware {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// State of one connected transmitter.
///
/// `level`, `serial`, and `firmware` are `None` when the model can tell a
/// transmitter is connected but hasn't learned its identity/level data yet.
/// On v2 firmware these arrive in separate periodic pushes from the compact
/// status heartbeat (see `PROTOCOL.md`), so there's a brief window after
/// connecting before they're known.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TxInfo {
    /// Live audio input level (raw device units).
    pub level: Option<u8>,
    pub serial: Option<String>,
    pub firmware: Option<Firmware>,
    /// Product name as the transmitter reports it, e.g. `"DJI Mic Mini"` or
    /// `"DJI Mic Mini 2"` — v2 firmware only (see `PROTOCOL.md`).
    pub product_name: Option<String>,
    /// Voice Tone (`"standard"`/`"rich"`/`"bright"`) — a DJI Mic Mini 2
    /// feature; other transmitters always report `"standard"` once
    /// connected on v2 firmware, `None` on v1 (see `PROTOCOL.md`).
    pub voice_tone: Option<String>,
    /// True while this transmitter is docked/charging (see `PROTOCOL.md`).
    /// Decoded on v2 firmware; not yet decoded on v1, though nothing about
    /// it suggests it's v2-specific, so this is always `None` there for now.
    pub charging: Option<bool>,
    /// Battery gauge, `1` (full) to `7` (empty, immediately precedes
    /// auto-shutoff) — see `PROTOCOL.md`. Higher means more depleted. `0`
    /// is believed unreachable in practice. Not every value is guaranteed
    /// to be observed during a fast charge from empty. `None` before the
    /// first status push for this transmitter, or on v1 firmware, which
    /// this isn't decoded on yet.
    pub battery: Option<u8>,
}

/// State of the receiver. `serial`/`firmware` are `None` when not yet known
/// (see [`TxInfo`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RxInfo {
    pub serial: Option<String>,
    pub firmware: Option<Firmware>,
}

/// A model-agnostic snapshot of a device's current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceStatus {
    /// Which [`DeviceModel`](crate::models::DeviceModel) produced this.
    pub model_id: String,
    /// True once at least one valid heartbeat has been decoded.
    pub connected: bool,
    /// Noise cancellation enabled (toggled by the physical button on the TX).
    pub nc_enabled: bool,
    /// Transmitter slots. `None` = that transmitter is powered off.
    pub tx: [Option<TxInfo>; 2],
    /// Receiver info, once known.
    pub rx: Option<RxInfo>,
    /// Current value slug of every writable setting, keyed by setting id.
    pub settings: BTreeMap<String, String>,
    /// The wire protocol version in use, once known from a decoded frame:
    /// `1` for v1 firmware, `2` for v2 firmware (see `PROTOCOL.md`'s
    /// "Protocol versions" section). `None` before the first frame decodes.
    pub protocol_version: Option<u8>,
    /// The receiver's physical gain dial, in dB (see `PROTOCOL.md`). Decoded
    /// on v2 firmware; not yet located in v1's heartbeat, so always `None`
    /// there for now.
    pub gain_dial: Option<i8>,
}

impl DeviceStatus {
    /// An empty, disconnected status for the given model.
    pub fn disconnected(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            connected: false,
            nc_enabled: false,
            tx: [None, None],
            rx: None,
            settings: BTreeMap::new(),
            protocol_version: None,
            gain_dial: None,
        }
    }

    /// Number of powered-on transmitters.
    pub fn tx_count(&self) -> usize {
        self.tx.iter().filter(|t| t.is_some()).count()
    }
}
