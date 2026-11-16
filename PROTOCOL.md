# DJI Mic Mini USB protocol

The DJI Mic Mini is controlled over USB Bulk transfers on **interface 6**
(vendor-specific). Two endpoints carry all traffic:

- **Bulk OUT `0x06`** — host → device (commands)
- **Bulk IN `0x86`** — device → host (acknowledgements and heartbeats)

USB identifiers: **vendor `0x2ca3`, product `0x4011`**.

Multi-byte integer fields are little-endian unless stated otherwise.

## Protocol versions

A firmware update changed the wire protocol: a different CRC-16 seed, a
target-addressed command shape, a restructured heartbeat, and several new
settings. The prior protocol is referred to as **v1** below and the new one
as **v2**; they otherwise share the same framing and the same 14-byte ACK. A
device's version is detected from its heartbeat stream (see
[Heartbeat marker](#heartbeat-marker-and-version-detection)) — there is no way
to ask for it directly.

Everything under "v1" and "v2" headings below is version-specific; everything
else applies to both.

---

## Packet framing

Every packet uses the DUML framing convention:

- Byte `0` is `0x55`, the start-of-frame marker.
- Byte `1` is the total packet length in bytes, including the header and the
  two-byte CRC at the end.
- Byte `2` is the protocol version, always `0x04`.
- Byte `3` is a CRC-8 (polynomial `0x31`, initial value `0xEE`, reflected
  in/out) computed over bytes `0`–`2`. Since those three bytes are otherwise
  fixed, this works out to a value that depends only on the packet's length
  (e.g. `0x03` for a 19-byte v1 command, `0xfc` for a 22-byte v2 one) — **it
  is not a routing/direction field**, despite v1 firmware only ever emitting
  two lengths (19 and 14) and thus only ever showing two values here. v2
  firmware's wider variety of packet lengths makes the actual pattern visible.
- The final two bytes are a CRC-16 (see below).

To read a stream, scan for `0x55` whose offset-2 byte is `0x04`, then take
`length` bytes.

---

## CRC-16

Packets end with a 2-byte CRC-16 built on the reflected `0x8408` polynomial
(same polynomial as CRC-16/IBM-SDLC / X-25). The two CRC bytes are chosen so
that checksumming the *entire* packet — body and CRC bytes together — always
lands on a fixed residue, for any content. This lets both directions validate
received packets and compute a checksum for outgoing ones without inspecting
a candidate CRC value directly.

| Version | Initial value | Residue  |
| ------- | -------------- | -------- |
| v1      | `0x0000`       | `0xbb01` |
| v2      | `0x3692`       | `0x0000` |

The polynomial and table are identical between versions; only the seed (and
therefore the residue it produces) changed.

---

## Command packets (host → device)

Sent to change a setting. The device replies with a [14-byte ACK](#ack-packets-device--host)
in both versions.

### v1

**Length: 19 bytes.** Example (Noise Cancel → Strong):

```
55 13 04 03 02 5a 01 00 40 5b 01 00 03 1d 00 01 01 91 fb
```

| Offset | Len | Value    | Description                                   |
| ------ | --- | -------- | --------------------------------------------- |
| 0      | 1   | `0x55`   | Start of frame                                |
| 1      | 1   | `0x13`   | Packet length (19)                            |
| 2      | 1   | `0x04`   | Protocol version                              |
| 3      | 1   | `0x03`   | Header CRC-8 for a 19-byte frame               |
| 4      | 1   | `0x02`   | Command set (audio subsystem)                 |
| 5      | 1   | `0x5a`   | Attribute flags                               |
| 6      | 2   | seq LE   | Sequence number, increments per command       |
| 8      | 1   | `0x40`   | ACK requested                                 |
| 9      | 1   | `0x5b`   | Constant                                      |
| 10     | 1   | `0x01`   | Constant                                      |
| 11     | 1   | `0x00`   | Constant                                      |
| 12     | 2   | cmd BE   | Command id, 16-bit **big-endian**             |
| 14     | 1   | `0x00`   | Constant                                      |
| 15     | 1   | `0x01`   | Payload length (1)                            |
| 16     | 1   | value    | Command value                                 |
| 17     | 2   | CRC      | See above                                     |

### v2

**Length: 22 bytes.** Same preamble as v1, but with an extra sub-type byte,
a 16-bit **target** address, and the command id written **little-endian**
instead of big-endian. Example (Noise Cancel → Basic, broadcast to both TX):

```
55 16 04 fc 02 5a 40 b6 40 5b 01 02 ff ff 00 00 37 00 01 00 dd e2
```

| Offset | Len | Value    | Description                                        |
| ------ | --- | -------- | --------------------------------------------------- |
| 0      | 1   | `0x55`   | Start of frame                                     |
| 1      | 1   | `0x16`   | Packet length (22)                                 |
| 2      | 1   | `0x04`   | Protocol version                                   |
| 3      | 1   | `0xfc`   | Header CRC-8 for a 22-byte frame                    |
| 4      | 1   | `0x02`   | Command set (audio subsystem)                       |
| 5      | 1   | `0x5a`   | Attribute flags                                    |
| 6      | 2   | seq LE   | Sequence number, increments per command             |
| 8      | 1   | `0x40`   | ACK requested                                      |
| 9      | 1   | `0x5b`   | Constant                                           |
| 10     | 1   | `0x01`   | Constant                                           |
| 11     | 1   | `0x02`   | Target-addressed command sub-type                   |
| 12     | 2   | target LE| `0x0000` = receiver, `0xffff` = broadcast to TX, or `0x0001`/`0x0002` for one specific TX unit (see Voice Tone) |
| 14     | 2   | `0x0000` | Constant                                           |
| 16     | 2   | cmd LE   | Command id, 16-bit **little-endian**                |
| 18     | 1   | `0x01`   | Payload length (1)                                  |
| 19     | 1   | value    | Command value                                      |
| 20     | 2   | CRC      | See above                                          |

### Commands

Every setting below works in both versions except the three marked "new" —
those don't exist on v1 firmware at all.

| Setting                              | v1 cmd     | v2 cmd       | Target      | Off/On values                    |
| ------------------------------------- | ---------- | ------------ | ----------- | --------------------------------- |
| Noise cancel mode (Basic/Strong)      | `0x031d`   | `0x0037`     | broadcast   | `0x00` Basic, `0x01` Strong       |
| Noise cancel enabled (**new**)        | —          | `0x0038`     | broadcast   | `0x00` off, `0x01` on             |
| NC toggle via TX button (**new**)     | —          | `0x000f`     | broadcast   | `0x00` off, `0x01` on             |
| Low cut                               | `0x0303`   | `0x0003`     | broadcast   | `0x00` off, `0x01` on             |
| Stereo tracking                       | `0x0008`   | `0x0008`     | receiver    | `0x00` off/mono, `0x02` on        |
| Safety track                          | `0x0021`   | `0x0021`     | receiver    | `0x00` off, `0x01` on             |
| Clip limiter                          | `0x001e`   | `0x001e`     | receiver    | `0x00` off, `0x01` on             |
| Auto off after 15 min (receiver)      | `0x0010`   | `0x0010`     | receiver    | `0x00` off, `0x01` on             |
| Auto off after 15 min (TX, **new**)   | —          | `0x0010`     | broadcast   | `0x00` off, `0x01` on             |
| Turn on with camera                   | `0x0020`   | `0x0020`     | receiver    | `0x00` off, `0x01` on             |
| Plug-free external speaker            | `0x0023`   | `0x0023`     | receiver    | `0x00` off, `0x01` on             |
| Mic LEDs                              | `0x030a`   | `0x000a`     | broadcast   | `0x00` on, `0x02` off (inverted)  |
| Voice Tone (**new**, Mic Mini 2 only)  | —          | `0x0029`     | one TX unit | `0x00` Standard, `0x01` Rich, `0x02` Bright |

Notes:

- Stereo tracking uses `0x02` for On (not `0x01`) — unchanged between versions.
- Mic LED values are inverted: `0x00` = On, `0x02` = Off — unchanged between
  versions, but v2 firmware issued it a new command id anyway.
- Toggling the plug-free external speaker reboots the receiver; the new state
  appears in heartbeats after the reboot completes.
- "NC toggle via TX button" does not itself enable/disable noise
  cancellation — it enables/disables whether a quick single press of the
  transmitter's power button can. Enabling/disabling NC itself is the
  separate "Noise cancel enabled" command above.
- Auto off (RX) and auto off (TX) share one command id, distinguished purely
  by target address — v1 firmware apparently only ever exposed the
  receiver's timer.
- Whether any setting besides noise cancel mode has a *different* command id
  between versions beyond what's listed here is unconfirmed — everything else
  reused its v1 id verbatim.
- Voice Tone is the only setting that targets one specific transmitter
  (`0x0001` or `0x0002`) rather than broadcasting to both — every other TX
  setting mirrors identically across whichever units are connected. It's
  also the only setting confirmed to exist on one hardware generation (DJI
  Mic Mini 2) but not another (the original DJI Mic Mini); every other
  setting has been exercised on both.

---

## ACK packets (device → host)

Returned for every command, in both versions, in the same 14-byte shape:

```
55 0e 04 66 5a 02 01 00 80 5b 01 00 c0 e8
```

| Offset | Len | Value  | Description                       |
| ------ | --- | ------ | ---------------------------------- |
| 0      | 1   | `0x55` | Start of frame                    |
| 1      | 1   | `0x0e` | Packet length (14)                |
| 2      | 1   | `0x04` | Protocol version                  |
| 3      | 1   | `0x66` | Header CRC-8 for a 14-byte frame  |
| 4      | 1   | `0x5a` | Attribute flags                   |
| 5      | 1   | `0x02` | Command set (audio subsystem)     |
| 6      | 2   | seq LE | Sequence echoed from the command  |
| 8      | 1   | `0x80` | ACK flag                          |
| 9      | 1   | `0x5b` | Constant                          |
| 10     | 1   | `0x01` | Constant                          |
| 11     | 1   | `0x00` | Constant                          |
| 12     | 2   | CRC    | See above                         |

---

## Heartbeat marker and version detection

Every heartbeat/status push shares this header shape at offset 8–11
regardless of version or sub-type:

| Offset | Len | Value      | Description                                    |
| ------ | --- | ---------- | ----------------------------------------------- |
| 8      | 1   | `0x00`     | No ACK required                                 |
| 9      | 2   | `5b 03`    | Constant                                        |
| 11     | 1   | `0x00`/`0x03` | **Version marker**: `0x00` = v1, `0x03` = v2 |

This is the only reliable way to tell which version a connected device speaks.

---

## v1 heartbeat

Streamed continuously (~10 Hz) on the Bulk IN endpoint, carrying current
state and per-transmitter data in one push. Packet size depends on how many
transmitters are powered on:

| Variant | Magic         | Length   | TX slots            |
| ------- | ------------- | -------- | ------------------- |
| 0-TX    | `55 38 04 e1` | 56 bytes | both absent         |
| 1-TX    | `55 46 04 8b` | 70 bytes | one present, one stub |
| 2-TX    | `55 54 04 f6` | 84 bytes | both present        |

All variants share a 14-byte header, two TX slots, a 22-byte RX entry, and a CRC.

### Header (14 bytes)

| Offset | Len | Value       | Description                                   |
| ------ | --- | ----------- | --------------------------------------------- |
| 0      | 1   | `0x55`      | Start of frame                                |
| 1      | 1   | length      | `0x38` / `0x46` / `0x54`                       |
| 2      | 1   | `0x04`      | Protocol version                              |
| 3      | 1   | type        | `0xe1` / `0x8b` / `0xf6` (header CRC-8, incidentally encodes TX count via length) |
| 4      | 1   | `0x5a`      | Attribute flags                               |
| 5      | 1   | `0x02`      | Command set (audio subsystem)                 |
| 6      | 2   | seq LE      | Sequence number                               |
| 8      | 1   | `0x00`      | No ACK required                               |
| 9      | 3   | `5b 03 00`  | Heartbeat marker (version = v1)               |
| 12     | 1   | —           | Device sub-state                              |
| 13     | 1   | `0x00`      | Constant                                      |

### TX slots

Starting at offset 14, each of the two slots is either a **present entry**
(23 bytes) or an **absent stub** (9 bytes). Byte `+1` of a slot distinguishes
them: `& 0x20` set → present, `& 0x40` set → absent. Slot 0 is TX1, slot 1 is TX2.

**Present TX entry (23 bytes):**

| Slot offset | Len | Description                                                    |
| ----------- | --- | ---------------------------------------------------------------- |
| +0          | 1   | State byte (see below)                                         |
| +1          | 1   | TX flags (see below)                                           |
| +2          | 1   | Audio input level                                              |
| +3          | 1   | LED flags — bit `0x80` set = LEDs off                          |
| +4          | 4   | Firmware version (one byte per component, decimal)             |
| +8          | 1   | Constant `0x0e`                                                |
| +9          | 14  | Transmitter serial number (ASCII)                              |

**Absent TX stub (9 bytes):** `+0` state byte, `+1` TX flags, `+2`..`+8` zero.
The state and flags bytes are valid even in a stub.

**State byte (`+0`):**

| Mask   | Meaning                                  |
| ------ | ----------------------------------------- |
| `0x20` | Strong noise-cancel mode (clear = Basic)  |
| `0x04` | Low cut active                           |

**TX flags byte (`+1`):**

| Mask   | Meaning                                              |
| ------ | ------------------------------------------------------ |
| `0x20` | Present TX marker                                     |
| `0x40` | Absent stub marker                                    |
| `0x01` | Noise cancel enabled (toggled by the TX's button)      |

### RX entry (22 bytes)

Follows the two TX slots (absolute offset 32 / 46 / 60 for 0 / 1 / 2-TX).

| RX offset | Len | Description                                        |
| --------- | --- | ---------------------------------------------------- |
| +0        | 1   | RX flags byte 0 (see below)                        |
| +1        | 1   | RX flags byte 1 (see below)                        |
| +2        | 4   | Firmware version (one byte per component, decimal) |
| +6        | 1   | Constant `0x0e`                                    |
| +7        | 14  | Receiver serial number (ASCII)                     |
| +21       | 1   | Plug-free external speaker: `0x01` on, `0x00` off  |

**RX flags byte 0:**

| Mask   | Meaning                    |
| ------ | ---------------------------- |
| `0x01` | Turn on with camera active |
| `0x02` | Auto off after 15 min active |
| `0x08` | Stereo tracking active     |

**RX flags byte 1:**

| Mask   | Meaning              |
| ------ | ---------------------- |
| `0x80` | Safety track active  |
| `0x20` | Clip limiter active  |

### Example: 2-TX heartbeat (84 bytes, serials are placeholder values)

```
55 54 04 f6 5a 02 0e 79 00 5b 03 00 47 00        header
b0 25 38 00 01 01 00 38 0e 42 42 42 42 42 42 42 42 42 42 42 42 42 42   TX1
b0 25 38 00 01 01 00 38 0e 43 43 43 43 43 43 43 43 43 43 43 43 43 43   TX2
32 20 01 01 00 38 0e 41 41 41 41 41 41 41 41 41 41 41 41 41 41 00   RX
<crc>
```

Decoded:

- TX1 — state `0xb0` (Strong NC, low cut off), level `0x38`, serial `BBBBBBBBBBBBBB`
  (placeholder), firmware `01.01.00.56`.
- TX2 — serial `CCCCCCCCCCCCCC` (placeholder).
- RX — flags `0x32`/`0x20` (auto-off on, clip on, others off), serial
  `AAAAAAAAAAAAAA` (placeholder), plug-free off.

---

## v2 status push

v2 firmware splits what v1 firmware pushed as one heartbeat into **three
separate periodic pushes** sharing the marker described above
([offset 11 = `0x03`](#heartbeat-marker-and-version-detection)):

1. **Status push** (this section) — compact flags plus a fixed-size slot per
   connected transmitter. Streamed continuously (~10 Hz), like the v1
   heartbeat.
2. **Identity push** — receiver/transmitter serial and product name, TLV-
   encoded. Sent once at connect (and again after a reconnect, e.g. the
   reboot a plug-free toggle causes), not on every tick.
3. **Audio-level push** — live per-transmitter input level, sent at roughly
   the same ~10 Hz rate as the status push but as its own message.

Frame *length alone* doesn't identify which of the three a given push is —
the identity push's length varies with how many units it lists, and can
coincide with a length the status push also uses — so decoding tries the
status push's fixed shape first and falls back to parsing the other two as
generic records (below); an unrecognised frame simply matches neither and is
ignored.

Since identity and level arrive on their own pushes rather than inside the
status push, a freshly connected device reports serial/firmware/level as
unknown for a moment until those pushes arrive; from then on, each new status
push must carry forward whatever was already learned rather than resetting it
to unknown on every tick (only dropping it if that transmitter disconnects).

### Status push shape

A fixed 52-byte header, then one 32-byte slot per **connected** transmitter
(0, 1, or 2 — there is no "absent stub" the way v1 firmware has; a
disconnected transmitter simply has no slot at all), then a 2-byte CRC:

| Variant | Length   | TX slots |
| ------- | -------- | -------- |
| 0-TX    | 54 bytes | none     |
| 1-TX    | 86 bytes | one      |
| 2-TX    | 118 bytes| two      |

Header offsets that matter (the rest is currently-unidentified constants):

| Offset | Len | Description                                                          |
| ------ | --- | ----------------------------------------------------------------------- |
| 0–11   | 12  | Standard preamble; see [framing](#packet-framing) and [marker](#heartbeat-marker-and-version-detection) |
| 12     | 1   | Capability byte: `0x26`/`0x46`/`0x66` for 0/1/2 TX connected (redundant with frame length) |
| 20     | 1   | Flags — see below                                                      |
| 21     | 1   | Flags — see below                                                      |
| 22     | 1   | Receiver's physical gain dial, signed, in dB — see below              |
| 44     | 1   | Per-TX connected bitmask — see below                                  |
| 48     | 1   | Flags — see below                                                      |
| 52     | 32×N| One slot per connected TX (see below)                                  |

**Offset 20:**

| Mask   | Meaning                              |
| ------ | --------------------------------------- |
| `0x08` | Strong noise-cancel mode (clear = Basic) |
| `0x20` | Noise cancel enabled                    |
| `0x80` | Turn on with camera active              |

**Offset 21:**

| Mask   | Meaning                    |
| ------ | ---------------------------- |
| `0x01` | Auto off (receiver) active |
| `0x04` | Stereo tracking active     |

**Offset 22:** the receiver's physical gain dial, a signed byte read directly
as dB — no scaling. The dial steps in fixed 6 dB increments across five
positions: `12`, `6`, `0`, `-6`, `-12`.

**Offset 44:** per-TX connected bitmask — `0x01` = TX1, `0x02` = TX2 (physical
unit numbering, not slot position). This is the authoritative signal for
which transmitter(s) are connected: it's set the instant a TX connects, up
to one status-push tick *before* that TX's 32-byte slot appears in the slot
list below, never the reverse — a decoder relying on slot presence alone
lags this bitmask by that one tick.

**Offset 48:**

| Mask   | Meaning                  |
| ------ | --------------------------- |
| `0x02` | Plug-free speaker active  |
| `0x10` | Clip limiter active      |
| `0x40` | Safety track active      |

**TX slot (32 bytes, at absolute offset `52 + 32×i`):**

The slot turns out to be a 6-byte record header — the same
`[tag:1][unit index:1][reserved:3][length:1]` shape used by the identity/
audio-level records below — followed by 26 bytes of data, of which only 4
are understood. Low Cut, Mic LEDs, TX Auto Off, and the NC-button permission
have **no header-level fallback** — they're only knowable when at least one
TX is connected, since they're purely per-transmitter settings. Everything
in this slot mirrors identically across whichever TX are present **except**
Voice Tone (`+9` bits `0x40`/`0x80`), the charging indicator (`+7` bit
`0x02`), and battery level (`+7` bits `0x1c`) — all independent
per transmitter, since each TX has its own tone setting, its own physical
dock/charge state, and its own battery level. See the Voice Tone note in
Commands above.

| Slot offset | Len | Description                                                        |
| ----------- | --- | --------------------------------------------------------------------- |
| +0          | 1   | Constant `0x02` (record tag)                                          |
| +1          | 1   | **Physical TX unit: `0x01` = TX1, `0x02` = TX2** — independent of slot position |
| +2          | 3   | Constant zero (reserved, matching the record header shape)            |
| +5          | 1   | Constant `0x1a` (26) — length of the data that follows, i.e. the rest of the slot |
| +6          | 1   | Flags: `0x02` LEDs off (inverted), `0x10` TX auto-off active, `0x20` strong NC (mirror), `0x80` NC-button-toggle permission active |
| +7          | 1   | Flags: `0x01` NC enabled (mirror of offset-20 bit `0x20`), `0x02` this TX is docked/charging (per-TX, not mirrored — reverts within a tick or two of undocking), `0x1c` battery level as a 3-bit gauge (per-TX — see below); constant bit `0x20` also set, purpose unknown |
| +8          | 1   | Flags: `0x04` clip limiter active (mirror of offset-48 bit `0x10`)    |
| +9          | 1   | Flags: `0x20` low cut active, `0x40` Voice Tone = Rich, `0x80` Voice Tone = Bright (neither set = Standard; Mic Mini 2 only — see Commands); a constant bit `0x01` also set, purpose unknown |
| +10         | 1   | Constant zero                                                          |
| +11         | 1   | Constant `0x78`, purpose unknown                                      |
| +12–+31     | 20  | Constant zero                                                         |

Since offset `+1` gives the physical TX directly, a decoder can place each
slot's data by real unit number and never needs to guess which physical
transmitter a lone connected slot belongs to.

**Battery level (`+7` bits `0x1c`, i.e. `0x04`/`0x08`/`0x10` together):**
independent per transmitter, and independent of the charging bit — both can
be set at once. These three bits form a single 3-bit gauge rather than
independent flags: read as `(byte >> 2) & 0x07`, higher means more
depleted. A value of `7` (all three bits set) is the terminal reading
immediately before the transmitter shuts off from an empty battery; `1`
(just `0x04`) corresponds to a full charge. The official app's low-battery
warning icon corresponds to a reading of `6` (`0x08` and `0x10` set, `0x04`
clear).

A reading of `0` (no bits set) has not been demonstrated to occur during
normal charge/discharge and is believed unreachable — the practical range
is `1`–`7`. The gauge does not update on every possible value during a fast
charge from empty: at least one intermediate value has been observed
skipped when charging resumes quickly after a full discharge, so a decoder
should not assume every value in range is guaranteed to appear.

### Identity and audio-level pushes: shared record format

Both push types are a sequence of records starting at offset 14, preceded by
a 16-bit little-endian length field at offset 12 giving the byte length of
everything from offset 14 up to (not including) the trailing CRC — i.e.
`frame length − 14 − 2`, redundant with the frame's own length:

```
[tag: 1][unit index: 1][reserved: 3][length: 1][data: length bytes]
```

repeated until the frame ends. Known tags:

| Tag    | Push     | Data                                                              | Unit index                          |
| ------ | -------- | ------------------------------------------------------------------ | -------------------------------------- |
| `0x01` | Identity | 4-byte firmware version (component order reversed — see below), then a 14-byte ASCII serial | `0` = receiver, `1`/`2` = TX slot 0/1 (0-based) |
| `0x04` | Identity | 6-character hex-like id (not modelled)                              | same as `0x01`                         |
| `0x06` | Identity | ASCII product name, e.g. `DJI Mic Mini` or `DJI Mic Mini 2` — variable length, matching the record's own length byte | same as `0x01`; only TX (`1`/`2`) is modelled |
| `0x05` | Level    | 1-byte live audio input level                                       | `1`/`2` = TX slot 0/1 (**1-based**, unlike the identity push) |

The `0x01` record's firmware bytes read `[b0, b1, b2, b3]` on the wire but
decode to version `b3.b2.b1.b0` (each byte is a decimal component, matching
the v1 firmware version encoding) — e.g. wire bytes `00 11 03 02` decode
to `02.03.17.00`.

The identity push is sent once at connect (and again after a reconnect, e.g.
the reboot a plug-free toggle causes) and periodically thereafter, listing
whichever units are currently connected: 70 bytes for the receiver alone, 124
bytes for the receiver plus one transmitter, **178 bytes for the receiver
plus both transmitters** — simply the same per-unit record block repeated
once per unit, each with its own index. 178 bytes is the largest
frame in either version: **`packet::take_frame`'s `MAX_FRAME` sanity limit
must stay above this**, since it was originally sized around v1's 84-byte
heartbeat and silently discarded this frame as an "implausible length" before
being raised. The level push is sent continuously (~10 Hz, like the status
push) and is a fixed 30 bytes for two transmitters.

Example identity push for a receiver-only connection (70 bytes, record
headers bolded via spacing; serial and hex id are placeholder values):

```
55 46 04 8b 5a 02 00 00 00 5b 03 03  36 00                              preamble
01 00 00 00 00  12  00 11 03 02 <serial: 14 bytes, "DDDDDDDDDDDDDD">    tag 0x01, unit 0, firmware 00 11 03 02
04 00 00 00  06  <hex id: 6 bytes>                                       tag 0x04, unit 0
06 00 00 00  0c  <product name: 12 bytes>                                tag 0x06, unit 0
<crc>
```

Firmware `00 11 03 02` decodes to `02.03.17.00`.

### Example: 2-TX status push (118 bytes)

```
55 76 04 a6 5a 02 00 00 00 5b 03 03 66 00 03 00 00 00 20 28 31 0c   header
00×22                                                                  (unidentified)
02 01 00 00 00 1a b8 25 04 01 00 78 00×20                             TX1
02 02 00 00 00 1a b8 25 04 01 00 78 00×20                             TX2
<crc>
```

Decoded: offset 20 = `0x28` (strong NC on, NC power on), offset 21 = `0x31`
(auto-off on, stereo off), offset 48 = `0x1c` (plug-free off, clip on, safety
off). Each TX slot opens `02 01`/`02 02` — record tag `0x02` plus the
physical unit (TX1/TX2) — followed by `00 00 00 1a` (reserved, then length
26). TX slot flags (+6) = `0xb8` (LEDs on, TX auto-off on, NC-button
permission on), low-cut byte (+9) = `0x01` (low cut off).
