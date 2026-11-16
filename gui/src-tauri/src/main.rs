// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod locale;

use std::sync::Arc;
use std::time::Duration;

use device::DeviceManager;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

/// Pick the device tray actions apply to: the sole device, else the first
/// connected one.
fn pick_device(mgr: &DeviceManager) -> Option<String> {
    if let Some(id) = mgr.only_device() {
        return Some(id);
    }
    mgr.list().into_iter().find(|d| d.connected).map(|d| d.id)
}

/// Flip a two-value setting: if it currently reads `a`, set `b`, otherwise `a`.
fn toggle_between(mgr: &DeviceManager, setting: &str, a: &str, b: &str) {
    let Some(id) = pick_device(mgr) else {
        return;
    };
    let current = mgr
        .status(&id)
        .ok()
        .and_then(|s| s.settings.get(setting).cloned());
    let next = if current.as_deref() == Some(a) { b } else { a };
    let _ = mgr.set(&id, setting, next);
}

/// Terminate the process, first SIGKILLing our child processes (the WebKitGTK
/// web/network helpers) so they die instantly instead of running their graceful
/// GL teardown, which segfaults in Mesa's EGL finalizer on some drivers.
#[cfg(unix)]
fn quit_now() -> ! {
    let me = std::process::id();
    if let Ok(dir) = std::fs::read_dir("/proc") {
        for entry in dir.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<i32>().ok()) else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                continue;
            };
            // `stat` is: pid (comm) state ppid ...; comm may contain spaces or
            // ')' so parse the fields after the final ')'.
            if let Some(rest) = stat.rsplit_once(')').map(|(_, r)| r) {
                let mut fields = rest.split_whitespace();
                let _state = fields.next();
                if fields.next().and_then(|s| s.parse::<u32>().ok()) == Some(me) {
                    unsafe { libc::kill(pid, libc::SIGKILL) };
                }
            }
        }
    }
    // Exit immediately without running our own finalizers (which would also try
    // to tear down GL) and before WebKit can respawn a helper.
    unsafe { libc::_exit(0) }
}

/// Show or hide the macOS Dock icon (no-op elsewhere). The app hides it while
/// running in the background with no window, like a menu-bar app.
fn set_dock_visible(app: &AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_dock_visibility(visible);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, visible);
    }
}

/// Reveal and focus the main window (and show the Dock icon).
fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        set_dock_visible(app, true);
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// What the tray menu reflects: whether to show the per-device toggles (exactly
/// one mic connected), the current NC mode, NC power, LED state, the connected
/// device's protocol version, and whether "Start at login" is enabled.
/// Compared each tick so the menu is only rebuilt on change.
type MenuState = (bool, Option<String>, Option<String>, Option<String>, Option<u8>, bool);

/// Read the current menu-relevant state.
fn menu_state(app: &AppHandle) -> MenuState {
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    if mgr.list().len() != 1 {
        return (false, None, None, None, None, autostart);
    }
    match pick_device(&mgr).and_then(|id| mgr.status(&id).ok()) {
        Some(st) => (
            true,
            st.settings.get("noise-cancel").cloned(),
            st.settings.get("noise-cancel-power").cloned(),
            st.settings.get("mic-leds").cloned(),
            st.protocol_version,
            autostart,
        ),
        None => (true, None, None, None, None, autostart),
    }
}

/// Build the tray menu. The per-device toggles appear only when exactly one mic
/// is connected, and their labels show the current mode and what tapping does.
fn build_menu(app: &AppHandle, state: &MenuState) -> tauri::Result<Menu<Wry>> {
    let (show, nc, nc_power, led, protocol_version, autostart) = state;
    let open = MenuItem::with_id(app, "open", locale::tr(app, "Open DJI Mic Control"), true, None::<&str>)?;
    let startup = CheckMenuItem::with_id(
        app,
        "toggle-autostart",
        locale::tr(app, "Start at login (hidden)"),
        true,
        *autostart,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", locale::tr(app, "Quit"), true, None::<&str>)?;
    let sep_a = PredefinedMenuItem::separator(app)?;
    let sep_b = PredefinedMenuItem::separator(app)?;

    if *show {
        let led_label = match led.as_deref() {
            Some("on") => locale::tr(app, "Mic LEDs: On  →  Off"),
            Some("off") => locale::tr(app, "Mic LEDs: Off  →  On"),
            _ => locale::tr(app, "Toggle Mic LEDs"),
        };
        let nc_label = match nc.as_deref() {
            Some("strong") => locale::tr(app, "Noise Cancel: Strong  →  Basic"),
            Some("basic") => locale::tr(app, "Noise Cancel: Basic  →  Strong"),
            _ => locale::tr(app, "Toggle NC Mode (Basic / Strong)"),
        };
        let leds = MenuItem::with_id(app, "toggle-leds", led_label, true, None::<&str>)?;
        let nc = MenuItem::with_id(app, "toggle-nc", nc_label, true, None::<&str>)?;
        let sep_c = PredefinedMenuItem::separator(app)?;

        // Enabling/disabling NC outright (as opposed to its Basic/Strong mode)
        // is only settable over the wire on v2 firmware — v1 only reports it,
        // toggled by the TX's physical button (see `noise-cancel-power` in
        // `settings.rs`).
        let nc_power_label = match nc_power.as_deref() {
            Some("on") => locale::tr(app, "Noise Cancelling: On  →  Off"),
            Some("off") => locale::tr(app, "Noise Cancelling: Off  →  On"),
            _ => locale::tr(app, "Toggle Noise Cancelling"),
        };
        let nc_power_item = (*protocol_version == Some(2))
            .then(|| MenuItem::with_id(app, "toggle-nc-power", nc_power_label, true, None::<&str>))
            .transpose()?;

        let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&open, &sep_a, &leds, &nc];
        if let Some(item) = &nc_power_item {
            items.push(item);
        }
        items.extend([&sep_b as &dyn IsMenuItem<Wry>, &startup, &sep_c, &quit]);
        Menu::with_items(app, &items)
    } else {
        Menu::with_items(app, &[&open, &sep_a, &startup, &sep_b, &quit])
    }
}

/// Swap the tray's menu to match current state (main thread only).
fn set_tray_menu(app: &AppHandle, state: &MenuState) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("main"), build_menu(app, state)) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// True when there's nothing to hear: no tracked mic, or every tracked mic is
/// present but not streaming ("No signal" in the device panel).
fn alert_active(app: &AppHandle) -> bool {
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    mgr.list().iter().all(|d| !d.connected)
}

/// True when a DJI Mic Mini 2 is connected to either TX slot of any tracked
/// device, so the tray icon can reflect it regardless of which physical slot
/// it's in.
fn mic_mini_2_present(app: &AppHandle) -> bool {
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    mgr.list().iter().any(|d| {
        mgr.status(&d.id).is_ok_and(|st| {
            st.tx.iter().flatten().any(|t| t.product_name.as_deref() == Some("DJI Mic Mini 2"))
        })
    })
}

/// Decode a PNG and scale it to fit within `max_dim`×`max_dim` (preserving
/// aspect ratio), for turning a full-size product photo into tray-icon art.
fn scaled_png(bytes: &[u8], max_dim: u32) -> Option<Image<'static>> {
    let resized = image::load_from_memory(bytes)
        .ok()?
        .resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let (width, height) = (resized.width(), resized.height());
    Some(Image::new_owned(resized.into_raw(), width, height))
}

/// Blend `color` into the pixel at `idx` by `coverage` (0..=1).
fn blend(rgba: &mut [u8], idx: usize, color: [u8; 4], coverage: f32) {
    let coverage = coverage.clamp(0.0, 1.0);
    for c in 0..4 {
        let src = color[c] as f32 * coverage;
        let dst = rgba[idx + c] as f32 * (1.0 - coverage);
        rgba[idx + c] = (src + dst).round().clamp(0.0, 255.0) as u8;
    }
}

/// Draw a filled, one-pixel-antialiased disc centered at (`cx`, `cy`).
fn draw_disc(rgba: &mut [u8], width: u32, height: u32, cx: f32, cy: f32, r: f32, color: [u8; 4]) {
    let (width, height) = (width as i32, height as i32);
    let x0 = ((cx - r).floor() as i32).max(0);
    let x1 = ((cx + r).ceil() as i32).min(width - 1);
    let y0 = ((cy - r).floor() as i32).max(0);
    let y1 = ((cy + r).ceil() as i32).min(height - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let coverage = r + 0.5 - (dx * dx + dy * dy).sqrt();
            if coverage > 0.0 {
                blend(rgba, ((y * width + x) * 4) as usize, color, coverage);
            }
        }
    }
}

/// Draw a round-capped line segment, used for the strokes of the "x".
fn draw_line(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    (x0, y0): (f32, f32),
    (x1, y1): (f32, f32),
    thickness: f32,
    color: [u8; 4],
) {
    let (width_i, height_i) = (width as i32, height as i32);
    let pad = thickness;
    let min_x = (x0.min(x1) - pad).floor().max(0.0) as i32;
    let max_x = ((x0.max(x1) + pad).ceil() as i32).min(width_i - 1);
    let min_y = (y0.min(y1) - pad).floor().max(0.0) as i32;
    let max_y = ((y0.max(y1) + pad).ceil() as i32).min(height_i - 1);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len_sq = dx * dx + dy * dy;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = if len_sq > 0.0 {
                (((px - x0) * dx + (py - y0) * dy) / len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (ex, ey) = (x0 + t * dx, y0 + t * dy);
            let dist = ((px - ex).powi(2) + (py - ey).powi(2)).sqrt();
            let coverage = thickness / 2.0 + 0.5 - dist;
            if coverage > 0.0 {
                blend(rgba, ((y * width_i + x) * 4) as usize, color, coverage);
            }
        }
    }
}

/// Worst-case battery state across every connected transmitter of every
/// tracked device. Drives the tray icon's low-battery warning: gauge value
/// `6` is where the transmitter's own LED and the main UI turn red, `7` is
/// the terminal reading immediately before auto-shutoff (see `PROTOCOL.md`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum BatteryAlert {
    Normal,
    Red,
    Critical,
}

fn battery_alert(app: &AppHandle) -> BatteryAlert {
    let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
    let mut worst = BatteryAlert::Normal;
    for d in mgr.list() {
        let Ok(st) = mgr.status(&d.id) else { continue };
        for tx in st.tx.iter().flatten() {
            match tx.battery {
                Some(7) => worst = BatteryAlert::Critical,
                Some(6) if worst == BatteryAlert::Normal => worst = BatteryAlert::Red,
                _ => {}
            }
        }
    }
    worst
}

/// Signed distance from `(px, py)` to a rounded rectangle at `(x0, y0)`
/// sized `w`×`h` with corner radius `r` (negative = inside). Standard
/// rounded-box SDF.
fn rounded_rect_sdf(px: f32, py: f32, x0: f32, y0: f32, w: f32, h: f32, r: f32) -> f32 {
    let r = r.max(0.0).min(w.min(h) / 2.0);
    let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
    let qx = (px - cx).abs() - w / 2.0 + r;
    let qy = (py - cy).abs() - h / 2.0 + r;
    let (mx, my) = (qx.max(0.0), qy.max(0.0));
    (mx * mx + my * my).sqrt() + qx.max(qy).min(0.0) - r
}

/// Overlay a small red "low battery" badge (a circle containing a tiny white
/// battery glyph) in the same bottom-right spot as the "no signal" badge
/// below -- the two are mutually exclusive in practice (no signal means no
/// transmitter is connected to have a battery reading from) so there's no
/// real conflict. Sits on top of whatever icon is already showing (the
/// generic logo or the Mic Mini 2 art) rather than replacing it, matching
/// how the "no signal" badge behaves.
fn battery_badge_icon(base: &Image<'static>) -> Image<'static> {
    const RED: [u8; 4] = [0xe0, 0x2b, 0x2b, 0xff];
    const WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

    let (width, height) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();

    let r = width.min(height) as f32 * 0.36;
    let (cx, cy) = (width as f32 - r * 0.95, height as f32 - r * 0.95);

    draw_disc(&mut rgba, width, height, cx, cy, r, WHITE);
    draw_disc(&mut rgba, width, height, cx, cy, r * 0.86, RED);

    // Mini battery glyph (ring + nub), white on the red disc.
    let body_w = r * 1.15;
    let body_h = r * 0.62;
    let x0 = cx - body_w / 2.0 - r * 0.05;
    let y0 = cy - body_h / 2.0;
    let rr = body_h * 0.24;
    let stroke = (r * 0.14).max(1.2);
    let nub_w = r * 0.13;
    let nub_h = body_h * 0.42;
    let nub_x0 = x0 + body_w - 1.0;
    let nub_y0 = cy - nub_h / 2.0;

    let x_min = (cx - r).max(0.0) as u32;
    let x_max = ((cx + r).min(width as f32 - 1.0)) as u32;
    let y_min = (cy - r).max(0.0) as u32;
    let y_max = ((cy + r).min(height as f32 - 1.0)) as u32;
    for y in y_min..=y_max {
        for x in x_min..=x_max {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let outer = rounded_rect_sdf(px, py, x0, y0, body_w, body_h, rr);
            let inner = rounded_rect_sdf(
                px,
                py,
                x0 + stroke,
                y0 + stroke,
                (body_w - 2.0 * stroke).max(0.0),
                (body_h - 2.0 * stroke).max(0.0),
                (rr - stroke).max(0.0),
            );
            let ring = outer.max(-inner);
            let nub = rounded_rect_sdf(px, py, nub_x0, nub_y0, nub_w, nub_h, nub_w.min(nub_h) * 0.3);
            let d = ring.min(nub);
            let coverage = (0.5 - d).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let idx = ((y * width + x) * 4) as usize;
                blend(&mut rgba, idx, WHITE, coverage);
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

/// Overlay a small red "no signal" badge (a circle with an "x") in the
/// bottom-right corner of `base`, returning a new owned icon.
fn badge_icon(base: &Image<'static>) -> Image<'static> {
    const RED: [u8; 4] = [0xe0, 0x2b, 0x2b, 0xff];
    const WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

    let (width, height) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();

    let r = width.min(height) as f32 * 0.30;
    let (cx, cy) = (width as f32 - r * 0.95, height as f32 - r * 0.95);

    draw_disc(&mut rgba, width, height, cx, cy, r, WHITE);
    draw_disc(&mut rgba, width, height, cx, cy, r * 0.86, RED);

    let arm = r * 0.5;
    let thickness = (r * 0.24).max(1.5);
    draw_line(&mut rgba, width, height, (cx - arm, cy - arm), (cx + arm, cy + arm), thickness, WHITE);
    draw_line(&mut rgba, width, height, (cx - arm, cy + arm), (cx + arm, cy - arm), thickness, WHITE);

    Image::new_owned(rgba, width, height)
}

/// The tray icon's four precomputed variants, selected by whether a Mic Mini
/// 2 is connected and whether the "no signal" badge should show.
struct TrayIcons {
    base: Option<Image<'static>>,
    base_alert: Option<Image<'static>>,
    mic2: Option<Image<'static>>,
    mic2_alert: Option<Image<'static>>,
}

impl TrayIcons {
    fn pick(&self, mic2: bool, alert: bool) -> Option<Image<'static>> {
        let icon = match (mic2, alert) {
            (true, true) => &self.mic2_alert,
            (true, false) => &self.mic2,
            (false, true) => &self.base_alert,
            (false, false) => &self.base,
        };
        icon.clone().or_else(|| self.base.clone())
    }

    /// The icon for a given moment: the usual mic2/no-signal variant, with
    /// the low-battery badge (see [`battery_badge_icon`]) layered on top
    /// rather than replacing it, so the app's own icon stays recognisable.
    /// At `Critical`, `blink_on` toggles the badge on and off each tick.
    fn pick_with_battery(
        &self,
        mic2: bool,
        alert: bool,
        battery: BatteryAlert,
        blink_on: bool,
    ) -> Option<Image<'static>> {
        let base = self.pick(mic2, alert)?;
        let badge = match battery {
            BatteryAlert::Normal => false,
            BatteryAlert::Red => true,
            BatteryAlert::Critical => blink_on,
        };
        Some(if badge { battery_badge_icon(&base) } else { base })
    }
}

fn main() {
    // On SIGTERM/SIGINT (logout, shutdown, Ctrl-C) exit via quit_now() so the
    // WebKit helpers are killed rather than crashing in their GL teardown. This
    // keeps the accelerated DMABUF renderer (full frame rate) without the noisy
    // exit-time segfaults.
    #[cfg(unix)]
    {
        use signal_hook::consts::{SIGINT, SIGTERM};
        if let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGTERM, SIGINT]) {
            std::thread::spawn(move || {
                if signals.forever().next().is_some() {
                    quit_now();
                }
            });
        }
    }

    let manager = Arc::new(DeviceManager::new());

    // Continuously rescan the bus so hotplugged devices appear/disappear.
    let scanner = manager.clone();
    std::thread::spawn(move || loop {
        scanner.refresh();
        std::thread::sleep(Duration::from_millis(1000));
    });

    // When launched at login/startup with --hidden, stay in the tray and don't
    // show the window.
    let start_hidden = std::env::args().skip(1).any(|a| a == "--hidden" || a == "--minimized");

    tauri::Builder::default()
        // Re-launching focuses the running window instead of starting a copy.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        // "Start at login" registers the app to autostart with --hidden.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .manage(locale::LocaleState::default())
        .manage(manager)
        .setup(move |app| {
            // Set the window icon explicitly so the titlebar and taskbar show it
            // even when the desktop-file association is unavailable (e.g. X11).
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) = Image::from_bytes(include_bytes!("../icons/128x128.png")) {
                    let _ = window.set_icon(icon);
                }
                // The window is created hidden; reveal it unless we started at login.
                if start_hidden {
                    set_dock_visible(app.handle(), false);
                } else {
                    let _ = window.show();
                }
            }

            // Four tray icon variants: the generic app logo, and a DJI Mic
            // Mini 2 variant swapped in when one's connected to either TX
            // slot (of any tracked device) — each with a pre-rendered "no
            // signal" badged counterpart.
            let tray_base = Image::from_bytes(include_bytes!("../icons/128x128.png")).ok();
            let tray_mic2 = scaled_png(
                include_bytes!("../../src/assets/devices/mic-mini-2-tx.png"),
                128,
            )
            .or_else(|| tray_base.clone());
            let icons = TrayIcons {
                base_alert: tray_base.as_ref().map(badge_icon),
                mic2_alert: tray_mic2.as_ref().map(badge_icon),
                base: tray_base,
                mic2: tray_mic2,
            };
            let initial_alert = alert_active(app.handle());
            let initial_mic2 = mic_mini_2_present(app.handle());
            let initial_battery = battery_alert(app.handle());

            let initial = menu_state(app.handle());
            let menu = build_menu(app.handle(), &initial)?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("DJI Mic Control")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "toggle-leds" => {
                        let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
                        std::thread::spawn(move || toggle_between(&mgr, "mic-leds", "on", "off"));
                    }
                    "toggle-nc" => {
                        let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
                        std::thread::spawn(move || {
                            toggle_between(&mgr, "noise-cancel", "strong", "basic")
                        });
                    }
                    "toggle-nc-power" => {
                        let mgr = app.state::<Arc<DeviceManager>>().inner().clone();
                        std::thread::spawn(move || {
                            toggle_between(&mgr, "noise-cancel-power", "on", "off")
                        });
                    }
                    "toggle-autostart" => {
                        let auto = app.autolaunch();
                        let _ = if auto.is_enabled().unwrap_or(false) {
                            auto.disable()
                        } else {
                            auto.enable()
                        };
                        // Reflect the new state in the menu's checkbox immediately.
                        set_tray_menu(app, &menu_state(app));
                    }
                    "quit" => {
                        #[cfg(unix)]
                        quit_now();
                        #[cfg(not(unix))]
                        app.exit(0);
                    }
                    _ => {}
                });
            let icon = icons
                .pick_with_battery(initial_mic2, initial_alert, initial_battery, true)
                .or_else(|| app.default_window_icon().cloned());
            if let Some(icon) = icon {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            // Keep the tray menu in sync with device count, NC/LED state, and the
            // autostart setting; keep the tray icon's "no signal" badge, Mic Mini 2
            // variant, and low-battery warning in sync with what's actually
            // connected. A critical battery blinks by repainting every tick rather
            // than only on change.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = initial;
                let mut last_alert = initial_alert;
                let mut last_mic2 = initial_mic2;
                let mut last_battery = initial_battery;
                let mut blink_on = true;
                loop {
                    let state = menu_state(&handle);
                    if state != last {
                        last = state.clone();
                        let h = handle.clone();
                        let _ = handle.run_on_main_thread(move || set_tray_menu(&h, &state));
                    }

                    let alert = alert_active(&handle);
                    let mic2 = mic_mini_2_present(&handle);
                    let battery = battery_alert(&handle);

                    let icon = if battery == BatteryAlert::Critical {
                        // Repaint every tick regardless of other state changes —
                        // that's what makes the badge blink on and off (the base
                        // icon underneath stays put either way).
                        blink_on = !blink_on;
                        icons.pick_with_battery(mic2, alert, battery, blink_on)
                    } else if battery != last_battery || alert != last_alert || mic2 != last_mic2 {
                        icons.pick_with_battery(mic2, alert, battery, true)
                    } else {
                        None
                    };
                    last_alert = alert;
                    last_mic2 = mic2;
                    last_battery = battery;

                    if let Some(icon) = icon {
                        let h = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            if let Some(tray) = h.tray_by_id("main") {
                                let _ = tray.set_icon(Some(icon));
                            }
                        });
                    }

                    std::thread::sleep(Duration::from_millis(1000));
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close to tray: hide the window (and the Dock icon) instead of
            // quitting the app.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                set_dock_visible(window.app_handle(), false);
            }
        })
        .invoke_handler(tauri::generate_handler![
            locale::set_locale,
            commands::snapshot,
            commands::set_setting,
            commands::set_tx_setting,
            commands::udev_help,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DJI Mic Control");
}
