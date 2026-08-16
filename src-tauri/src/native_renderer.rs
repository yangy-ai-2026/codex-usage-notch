use crate::engine::{CreditSnapshot, CreditStatus, RuntimeStatus, UsageSnapshot};
use crate::window_geometry::WindowSize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenderWindow {
    pub label: String,
    pub percentage: String,
    pub reset: String,
    pub remaining_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRenderModel {
    pub status: RuntimeStatus,
    pub credit_status: CreditStatus,
    pub expanded: bool,
    pub size: WindowSize,
    pub title: String,
    pub primary_percentage: String,
    pub status_label: String,
    pub credit_label: String,
    pub windows: Vec<NativeRenderWindow>,
}

pub fn build_render_model(
    snapshot: &UsageSnapshot,
    credits: &CreditSnapshot,
    expanded: bool,
    size: WindowSize,
) -> NativeRenderModel {
    let primary_percentage = match snapshot.windows.first() {
        Some(window)
            if !matches!(
                snapshot.status,
                RuntimeStatus::Unavailable | RuntimeStatus::Error
            ) =>
        {
            format!("{}%", window.remaining_percent)
        }
        _ => "--".to_string(),
    };

    NativeRenderModel {
        status: snapshot.status.clone(),
        credit_status: credits.status.clone(),
        expanded,
        size,
        title: if expanded {
            "Codex Usage".to_string()
        } else {
            "Codex".to_string()
        },
        primary_percentage,
        status_label: status_label(&snapshot.status),
        credit_label: credit_label(credits),
        windows: snapshot
            .windows
            .iter()
            .map(|window| NativeRenderWindow {
                label: format_duration(window.window_duration_mins),
                percentage: format!("{}%", window.remaining_percent),
                reset: format_reset(window.resets_at.as_deref()),
                remaining_percent: window.remaining_percent,
            })
            .collect(),
    }
}

fn credit_label(snapshot: &CreditSnapshot) -> String {
    let balance = snapshot
        .balance
        .as_deref()
        .map(format_credit_balance)
        .unwrap_or_else(|| "—".to_string());
    match &snapshot.status {
        CreditStatus::Loading => "Credits — · Reading".to_string(),
        CreditStatus::Available => format!("Credits {balance}"),
        CreditStatus::Unlimited => "Credits ∞".to_string(),
        CreditStatus::Unavailable => "Credits —".to_string(),
        CreditStatus::Stale => format!("Credits {balance} · Stale"),
        CreditStatus::Error => "Credits — · Error".to_string(),
    }
}

fn format_credit_balance(balance: &str) -> String {
    let Ok(value) = balance.parse::<f64>() else {
        return balance.to_string();
    };
    if !value.is_finite() {
        return balance.to_string();
    }

    let formatted = format!("{value:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn status_label(status: &RuntimeStatus) -> String {
    match status {
        RuntimeStatus::Loading => "Reading".to_string(),
        RuntimeStatus::Fresh => "Fresh".to_string(),
        RuntimeStatus::Partial => "Partial".to_string(),
        RuntimeStatus::Stale => "Stale".to_string(),
        RuntimeStatus::Unavailable => "Unavailable".to_string(),
        RuntimeStatus::Error => "Error".to_string(),
    }
}

fn format_duration(minutes: u64) -> String {
    if minutes % 10080 == 0 {
        return format!(
            "{} week{}",
            minutes / 10080,
            if minutes == 10080 { "" } else { "s" }
        );
    }
    if minutes % 1440 == 0 {
        return format!(
            "{} day{}",
            minutes / 1440,
            if minutes == 1440 { "" } else { "s" }
        );
    }
    if minutes % 60 == 0 {
        return format!("{}h", minutes / 60);
    }
    format!("{}m", minutes)
}

fn format_reset(value: Option<&str>) -> String {
    let Some(seconds) = value.and_then(|value| value.parse::<i64>().ok()) else {
        return "Reset unavailable".to_string();
    };
    let days = seconds.div_euclid(86_400);
    let (year, month, day) = civil_date_from_days(days);
    format!("Reset {year:04}-{month:02}-{day:02}")
}

fn civil_date_from_days(days: i64) -> (i64, u32, u32) {
    let adjusted = days + 719_468;
    let era = if adjusted >= 0 {
        adjusted / 146_097
    } else {
        (adjusted - 146_096) / 146_097
    };
    let day_of_era = adjusted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month as u32, day as u32)
}

#[cfg(windows)]
pub fn render_layered_window(
    hwnd: windows::Win32::Foundation::HWND,
    model: &NativeRenderModel,
) -> bool {
    use std::ffi::c_void;
    use windows::Win32::Foundation::{COLORREF, POINT, SIZE};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, SelectObject,
        AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
        DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};

    let width = model.size.width.max(1) as usize;
    let height = model.size.height.max(1) as usize;
    let screen_dc = unsafe { GetDC(None) };
    if screen_dc.is_invalid() {
        return false;
    }
    let memory_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if memory_dc.is_invalid() {
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
        }
        return false;
    }

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: model.size.width.max(1),
            biHeight: -model.size.height.max(1),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let Ok(bitmap) = (unsafe {
        CreateDIBSection(
            Some(screen_dc),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )
    }) else {
        unsafe {
            let _ = DeleteDC(memory_dc);
            let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
        }
        return false;
    };

    let old_bitmap = unsafe { SelectObject(memory_dc, HGDIOBJ(bitmap.0)) };
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits as *mut u32, width * height) };
    draw_surface(pixels, model.size.width, model.size.height, model);
    draw_text_surface(memory_dc, model);
    for pixel in pixels.iter_mut() {
        if (*pixel & 0x00ff_ffff) != 0 && (*pixel >> 24) == 0 {
            *pixel |= 0xff00_0000;
        }
    }

    let destination = POINT { x: 0, y: 0 };
    let source = POINT { x: 0, y: 0 };
    let size = SIZE {
        cx: model.size.width.max(1),
        cy: model.size.height.max(1),
    };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let updated = unsafe {
        UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&destination as *const _),
            Some(&size as *const _),
            Some(memory_dc),
            Some(&source as *const _),
            COLORREF(0),
            Some(&blend as *const _),
            ULW_ALPHA,
        )
        .is_ok()
    };

    unsafe {
        let _ = SelectObject(memory_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
        let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
    }
    updated
}

#[cfg(not(windows))]
pub fn render_layered_window(_hwnd: u64, _model: &NativeRenderModel) -> bool {
    false
}

#[cfg(windows)]
fn draw_surface(pixels: &mut [u32], width: i32, height: i32, model: &NativeRenderModel) {
    let scale = (width.max(1) as f32 / 480.0).max(1.0);
    let radius = scaled(20, scale);
    let border = premultiplied_bgra(242, 184, 75, 220);
    let background = premultiplied_bgra(28, 25, 22, 245);
    fill_rounded(pixels, width, height, 0, 0, width, height, radius, border);
    fill_rounded(
        pixels,
        width,
        height,
        scaled(2, scale),
        scaled(2, scale),
        width - scaled(2, scale),
        height - scaled(2, scale),
        (radius - scaled(2, scale)).max(1),
        background,
    );

    let mark_size = scaled(8, scale);
    let mark_x = scaled(22, scale);
    let mark_y = if model.expanded {
        scaled(16, scale)
    } else {
        scaled(28, scale)
    };
    fill_rect(
        pixels,
        width,
        height,
        mark_x,
        mark_y,
        mark_x + mark_size,
        mark_y + mark_size,
        premultiplied_bgra(242, 184, 75, 255),
    );

    if model.expanded {
        for (index, window) in model.windows.iter().enumerate() {
            let y = scaled(28 + index as i32 * 34, scale);
            let bar_left = scaled(22, scale);
            let bar_right = width - scaled(22, scale);
            let bar_top = y + scaled(28, scale);
            let bar_bottom = bar_top + scaled(4, scale);
            fill_rect(
                pixels,
                width,
                height,
                bar_left,
                bar_top,
                bar_right,
                bar_bottom,
                premultiplied_bgra(64, 56, 48, 255),
            );
            let filled =
                bar_left + ((bar_right - bar_left) * i32::from(window.remaining_percent) / 100);
            fill_rect(
                pixels,
                width,
                height,
                bar_left,
                bar_top,
                filled.max(bar_left),
                bar_bottom,
                premultiplied_bgra(242, 184, 75, 255),
            );
        }
    }
}

#[cfg(windows)]
fn draw_text_surface(hdc: windows::Win32::Graphics::Gdi::HDC, model: &NativeRenderModel) {
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, DeleteObject, DrawTextW, SelectObject, SetBkMode, SetTextColor,
        CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_NOPREFIX,
        DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_BOLD, FW_NORMAL, HGDIOBJ, OUT_DEFAULT_PRECIS,
        TRANSPARENT,
    };

    let scale = (model.size.width.max(1) as f32 / 480.0).max(1.0);
    let title_size = scaled(17, scale);
    let body_size = scaled(14, scale);
    let small_size = scaled(11, scale);
    let white = rgb(246, 242, 234);
    let muted = rgb(169, 160, 150);
    let accent = rgb(242, 184, 75);

    unsafe {
        let _ = SetBkMode(hdc, TRANSPARENT);
        if model.expanded {
            draw_text(
                hdc,
                &model.title,
                rect(
                    scaled(44, scale),
                    scaled(6, scale),
                    model.size.width - scaled(160, scale),
                    scaled(28, scale),
                ),
                title_size,
                white,
                true,
            );
            draw_text(
                hdc,
                &model.status_label,
                rect(
                    model.size.width - scaled(150, scale),
                    scaled(8, scale),
                    model.size.width - scaled(22, scale),
                    scaled(26, scale),
                ),
                small_size,
                muted,
                false,
            );
            for (index, window) in model.windows.iter().enumerate() {
                let y = scaled(28 + index as i32 * 34, scale);
                draw_text(
                    hdc,
                    &window.label,
                    rect(
                        scaled(22, scale),
                        y,
                        model.size.width - scaled(110, scale),
                        y + scaled(16, scale),
                    ),
                    body_size,
                    white,
                    false,
                );
                draw_text(
                    hdc,
                    &window.percentage,
                    rect(
                        model.size.width - scaled(100, scale),
                        y,
                        model.size.width - scaled(22, scale),
                        y + scaled(16, scale),
                    ),
                    body_size,
                    accent,
                    true,
                );
                draw_text(
                    hdc,
                    &window.reset,
                    rect(
                        scaled(22, scale),
                        y + scaled(15, scale),
                        model.size.width - scaled(22, scale),
                        y + scaled(28, scale),
                    ),
                    small_size,
                    muted,
                    false,
                );
            }
            if model.windows.is_empty() {
                draw_text(
                    hdc,
                    "No allowance window",
                    rect(
                        scaled(22, scale),
                        scaled(35, scale),
                        model.size.width - scaled(22, scale),
                        scaled(58, scale),
                    ),
                    body_size,
                    muted,
                    false,
                );
            }
            let credit_color = if matches!(
                &model.credit_status,
                CreditStatus::Available | CreditStatus::Unlimited
            ) {
                white
            } else {
                muted
            };
            draw_text(
                hdc,
                &model.credit_label,
                rect(
                    scaled(22, scale),
                    model.size.height - scaled(18, scale),
                    model.size.width - scaled(22, scale),
                    model.size.height - scaled(3, scale),
                ),
                small_size,
                credit_color,
                true,
            );
        } else {
            draw_text(
                hdc,
                &model.title,
                rect(scaled(44, scale), 0, scaled(110, scale), model.size.height),
                title_size,
                white,
                true,
            );
            draw_text(
                hdc,
                &model.primary_percentage,
                rect(scaled(118, scale), 0, scaled(170, scale), model.size.height),
                title_size,
                accent,
                true,
            );
            draw_text(
                hdc,
                &model.status_label,
                rect(scaled(178, scale), 0, scaled(224, scale), model.size.height),
                small_size,
                muted,
                false,
            );
            draw_text(
                hdc,
                "·",
                rect(scaled(226, scale), 0, scaled(236, scale), model.size.height),
                small_size,
                muted,
                false,
            );
            draw_text(
                hdc,
                &model.credit_label,
                rect(
                    scaled(238, scale),
                    0,
                    model.size.width - scaled(20, scale),
                    model.size.height,
                ),
                small_size,
                if matches!(
                    &model.credit_status,
                    CreditStatus::Available | CreditStatus::Unlimited
                ) {
                    white
                } else {
                    muted
                },
                true,
            );
        }
    }

    fn draw_text(
        hdc: windows::Win32::Graphics::Gdi::HDC,
        value: &str,
        mut bounds: RECT,
        size: i32,
        color: COLORREF,
        bold: bool,
    ) {
        let mut text: Vec<u16> = value.encode_utf16().collect();
        unsafe {
            let font = CreateFontW(
                -size,
                0,
                0,
                0,
                if bold {
                    FW_BOLD.0 as i32
                } else {
                    FW_NORMAL.0 as i32
                },
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
                windows::core::w!("Segoe UI"),
            );
            let previous = SelectObject(hdc, HGDIOBJ(font.0));
            let _ = SetTextColor(hdc, color);
            let _ = DrawTextW(
                hdc,
                &mut text,
                &mut bounds,
                DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
            );
            let _ = SelectObject(hdc, previous);
            let _ = DeleteObject(HGDIOBJ(font.0));
        }
    }

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    fn rgb(red: u32, green: u32, blue: u32) -> COLORREF {
        COLORREF(red | (green << 8) | (blue << 16))
    }
}

#[cfg(windows)]
fn scaled(value: i32, scale: f32) -> i32 {
    ((value as f32 * scale).round() as i32).max(1)
}

#[cfg(windows)]
fn fill_rect(
    pixels: &mut [u32],
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: u32,
) {
    let left = left.clamp(0, width);
    let top = top.clamp(0, height);
    let right = right.clamp(left, width);
    let bottom = bottom.clamp(top, height);
    for y in top..bottom {
        let row = y as usize * width as usize;
        for x in left..right {
            pixels[row + x as usize] = color;
        }
    }
}

#[cfg(windows)]
fn fill_rounded(
    pixels: &mut [u32],
    width: i32,
    height: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    radius: i32,
    color: u32,
) {
    let left = left.clamp(0, width);
    let top = top.clamp(0, height);
    let right = right.clamp(left, width);
    let bottom = bottom.clamp(top, height);
    for y in top..bottom {
        for x in left..right {
            if rounded_rect_contains(x - left, y - top, right - left, bottom - top, radius) {
                pixels[y as usize * width as usize + x as usize] = color;
            }
        }
    }
}

#[cfg(windows)]
fn rounded_rect_contains(x: i32, y: i32, width: i32, height: i32, radius: i32) -> bool {
    let radius = radius.min(width / 2).min(height / 2).max(1);
    let center_x = if x < radius {
        radius
    } else if x >= width - radius {
        width - radius - 1
    } else {
        x
    };
    let center_y = if y < radius {
        radius
    } else if y >= height - radius {
        height - radius - 1
    } else {
        y
    };
    let dx = x - center_x;
    let dy = y - center_y;
    dx * dx + dy * dy <= radius * radius
}

#[cfg(windows)]
fn premultiplied_bgra(red: u32, green: u32, blue: u32, alpha: u32) -> u32 {
    let red = red * alpha / 255;
    let green = green * alpha / 255;
    let blue = blue * alpha / 255;
    blue | (green << 8) | (red << 16) | (alpha << 24)
}

#[cfg(test)]
mod tests {
    use super::{build_render_model, format_credit_balance};
    use crate::engine::{CreditSnapshot, CreditStatus, RuntimeStatus, UsageSnapshot, UsageWindow};
    use crate::window_geometry::WindowSize;

    fn snapshot(status: RuntimeStatus, windows: Vec<UsageWindow>) -> UsageSnapshot {
        UsageSnapshot {
            windows,
            status,
            fetched_at: Some(1),
            last_successful_at: Some(1),
            source: "test".into(),
            capability: "account/rateLimits/read".into(),
            diagnostic_code: None,
        }
    }

    fn window(duration: u64, remaining: u8) -> UsageWindow {
        UsageWindow {
            key: format!("codex:{duration}"),
            limit_id: "codex".into(),
            window_duration_mins: duration,
            used_percent: 100 - remaining,
            remaining_percent: remaining,
            resets_at: None,
            source_slot: "primary".into(),
        }
    }

    fn credits(status: CreditStatus, balance: Option<&str>) -> CreditSnapshot {
        CreditSnapshot {
            has_credits: matches!(status, CreditStatus::Available | CreditStatus::Unlimited),
            unlimited: status == CreditStatus::Unlimited,
            balance: balance.map(str::to_string),
            status,
            fetched_at: Some(1),
            last_successful_at: Some(1),
            diagnostic_code: None,
        }
    }

    #[test]
    fn collapsed_model_uses_real_primary_remaining_percentage() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
            &credits(CreditStatus::Unavailable, None),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.primary_percentage, "42%");
        assert_eq!(model.status_label, "Partial");
    }

    #[test]
    fn unavailable_model_does_not_invent_a_percentage() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Unavailable, Vec::new()),
            &credits(CreditStatus::Unavailable, None),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.primary_percentage, "--");
        assert_eq!(model.status_label, "Unavailable");
    }

    #[test]
    fn expanded_model_preserves_dynamic_window_collection() {
        let model = build_render_model(
            &snapshot(
                RuntimeStatus::Fresh,
                vec![window(300, 80), window(10080, 42)],
            ),
            &credits(CreditStatus::Unavailable, None),
            true,
            WindowSize {
                width: 500,
                height: 112,
            },
        );
        assert_eq!(model.windows.len(), 2);
        assert_eq!(model.windows[0].label, "5h");
        assert_eq!(model.windows[1].label, "1 week");
    }

    #[test]
    fn stale_model_keeps_the_last_valid_window_value() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Stale, vec![window(10080, 0)]),
            &credits(CreditStatus::Unavailable, None),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.primary_percentage, "0%");
        assert_eq!(model.status_label, "Stale");
    }

    #[test]
    fn available_credit_balance_renders_exactly() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
            &credits(CreditStatus::Available, Some("827.9644120000")),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.credit_label, "Credits 827.96");
    }

    #[test]
    fn expanded_model_renders_compact_credit_row() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
            &credits(CreditStatus::Available, Some("841.5000")),
            true,
            WindowSize {
                width: 500,
                height: 112,
            },
        );
        assert_eq!(model.credit_label, "Credits 841.5");
        assert_eq!(
            model.size,
            WindowSize {
                width: 500,
                height: 112
            }
        );
    }

    #[test]
    fn unlimited_credit_balance_renders_separately() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Fresh, vec![window(10080, 42)]),
            &credits(CreditStatus::Unlimited, None),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.credit_label, "Credits ∞");
    }

    #[test]
    fn missing_and_null_credit_balance_render_honestly() {
        for status in [CreditStatus::Unavailable, CreditStatus::Error] {
            let model = build_render_model(
                &snapshot(RuntimeStatus::Fresh, vec![window(10080, 42)]),
                &credits(status, None),
                false,
                WindowSize {
                    width: 480,
                    height: 64,
                },
            );
            assert!(model.credit_label.contains("Credits —"));
            assert!(!model.credit_label.contains('0'));
        }
    }

    #[test]
    fn stale_credit_balance_preserves_last_value_and_marks_state() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
            &credits(CreditStatus::Stale, Some("841")),
            true,
            WindowSize {
                width: 500,
                height: 112,
            },
        );
        assert_eq!(model.credit_label, "Credits 841 · Stale");
        assert_eq!(model.status, RuntimeStatus::Partial);
        assert_eq!(model.credit_status, CreditStatus::Stale);
    }

    #[test]
    fn purchased_balance_label_does_not_use_reset_credit_count() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Fresh, vec![window(10080, 42)]),
            &credits(CreditStatus::Available, Some("841")),
            false,
            WindowSize {
                width: 480,
                height: 64,
            },
        );
        assert_eq!(model.credit_label, "Credits 841");
        assert!(!model.credit_label.contains("3"));
    }

    #[test]
    fn credit_balance_formatter_trims_to_two_decimal_places() {
        assert_eq!(format_credit_balance("827.9644120000"), "827.96");
        assert_eq!(format_credit_balance("841.5000"), "841.5");
        assert_eq!(format_credit_balance("841.0000"), "841");
    }

    #[test]
    fn invalid_credit_balance_falls_back_to_original_string() {
        assert_eq!(format_credit_balance("balance-unknown"), "balance-unknown");
        assert_eq!(format_credit_balance("NaN"), "NaN");
    }
}
