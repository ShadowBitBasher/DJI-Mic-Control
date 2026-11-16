//! Presentation-only locale state. Device commands and protocol values stay unchanged.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct LocaleState(pub AtomicBool);

pub fn tr<'a>(app: &AppHandle, english: &'a str) -> &'a str {
    if !app.state::<LocaleState>().0.load(Ordering::Relaxed) {
        return english;
    }
    match english {
        "Open DJI Mic Control" => "打开 DJI Mic Control",
        "Start at login (hidden)" => "登录时启动（隐藏窗口）",
        "Quit" => "退出",
        "Mic LEDs: On  →  Off" => "指示灯：开启 → 关闭",
        "Mic LEDs: Off  →  On" => "指示灯：关闭 → 开启",
        "Toggle Mic LEDs" => "切换指示灯",
        "Noise Cancel: Strong  →  Basic" => "降噪模式：强效 → 基础",
        "Noise Cancel: Basic  →  Strong" => "降噪模式：基础 → 强效",
        "Toggle NC Mode (Basic / Strong)" => "切换降噪模式（基础 / 强效）",
        "Noise Cancelling: On  →  Off" => "降噪：开启 → 关闭",
        "Noise Cancelling: Off  →  On" => "降噪：关闭 → 开启",
        "Toggle Noise Cancelling" => "切换降噪",
        _ => english,
    }
}

#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> Result<(), String> {
    let chinese = match locale.as_str() {
        "zh-CN" => true,
        "en" => false,
        _ => return Err("Unsupported UI locale".into()),
    };
    let handle = app.clone();
    app.run_on_main_thread(move || {
        handle
            .state::<LocaleState>()
            .0
            .store(chinese, Ordering::Relaxed);
        super::set_tray_menu(&handle, &super::menu_state(&handle));
        #[cfg(target_os = "macos")]
        if let Ok(menu) = macos_menu(&handle, chinese) {
            let _ = handle.set_menu(menu);
        }
    })
    .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn macos_menu(app: &AppHandle, chinese: bool) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItemKind};
    fn label(text: &str) -> String {
        match text {
            "File" => "文件",
            "Edit" => "编辑",
            "View" => "显示",
            "Window" => "窗口",
            "Help" => "帮助",
            "Services" => "服务",
            "Undo" => "撤销",
            "Redo" => "重做",
            "Cut" => "剪切",
            "Copy" => "复制",
            "Paste" => "粘贴",
            "Select All" => "全选",
            "Minimize" => "最小化",
            "Zoom" | "Maximize" => "缩放",
            "Close" | "Close Window" => "关闭窗口",
            "Enter Full Screen" | "Toggle Fullscreen" => "切换全屏",
            "Hide Others" => "隐藏其他应用",
            "Show All" => "全部显示",
            other => {
                for (prefix, translation) in
                    [("About ", "关于 "), ("Hide ", "隐藏 "), ("Quit ", "退出 ")]
                {
                    if let Some(name) = other.strip_prefix(prefix) {
                        return format!("{translation}{name}");
                    }
                }
                return other.to_string();
            }
        }
        .to_string()
    }
    fn translate_items(items: Vec<MenuItemKind<tauri::Wry>>) -> tauri::Result<()> {
        for item in items {
            match item {
                MenuItemKind::Submenu(menu) => {
                    menu.set_text(label(&menu.text()?))?;
                    translate_items(menu.items()?)?;
                }
                MenuItemKind::Predefined(item) => item.set_text(label(&item.text()?))?,
                _ => {}
            }
        }
        Ok(())
    }
    // Preserve default roles, shortcuts, IDs, Services, and About metadata.
    let menu = Menu::default(app)?;
    if chinese {
        translate_items(menu.items()?)?;
    }
    Ok(menu)
}
