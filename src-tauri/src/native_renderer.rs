use crate::engine::{CreditSnapshot, CreditStatus, RuntimeStatus, UsageSnapshot};
use crate::settings::Theme;
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
    pub theme: Theme,
    pub expanded: bool,
    pub edge_tab: bool,
    pub size: WindowSize,
    pub title: String,
    pub primary_percentage: String,
    pub status_label: String,
    pub credit_label: String,
    pub credit_value: Option<String>,
    pub windows: Vec<NativeRenderWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompactLayout {
    width: i32,
    icon_left: i32,
    icon_right: i32,
    title_left: i32,
    title_right: i32,
    percentage_left: i32,
    percentage_right: i32,
    credit_left: i32,
    credit_label_right: i32,
    credit_value_left: Option<i32>,
    credit_value_right: Option<i32>,
    credit_right: i32,
    separator_left: i32,
    separator_right: i32,
    status_left: i32,
    status_right: i32,
    chevron_center_x: i32,
}

const COMPACT_CREDIT_VALUE_RIGHT_PADDING: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExpandedRowLayout {
    top: i32,
    helper_top: i32,
    bar_top: i32,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThemePalette {
    edge_tab: [u8; 4],
    expanded_border: [u8; 4],
    compact_border: [u8; 4],
    expanded_background: [u8; 4],
    compact_background: [u8; 4],
    progress_track: [u8; 4],
    progress_fill: [u8; 4],
    divider: [u8; 4],
    compact_dot: [u8; 4],
    expanded_chevron: [u8; 4],
    compact_chevron: [u8; 4],
    primary: [u8; 3],
    muted: [u8; 3],
    accent: [u8; 3],
    lightning: [u8; 3],
    credit: [u8; 3],
    secondary: [u8; 3],
    expanded_helper: [u8; 3],
    expanded_status: [u8; 3],
}

#[cfg(windows)]
impl ThemePalette {
    fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                edge_tab: [196, 137, 42, 245],
                expanded_border: [198, 140, 48, 220],
                compact_border: [211, 153, 54, 235],
                expanded_background: [248, 246, 240, 245],
                compact_background: [252, 250, 245, 248],
                progress_track: [220, 214, 202, 255],
                progress_fill: [205, 143, 43, 255],
                divider: [218, 211, 198, 180],
                compact_dot: [145, 136, 120, 255],
                expanded_chevron: [193, 132, 37, 255],
                compact_chevron: [181, 124, 38, 255],
                primary: [47, 43, 38],
                muted: [105, 98, 88],
                accent: [193, 132, 37],
                lightning: [186, 123, 31],
                credit: [171, 116, 28],
                secondary: [120, 112, 100],
                expanded_helper: [111, 103, 92],
                expanded_status: [147, 139, 127],
            },
            Theme::Dark | Theme::System => Self {
                edge_tab: [247, 192, 87, 245],
                expanded_border: [242, 184, 75, 220],
                compact_border: [247, 192, 87, 235],
                expanded_background: [28, 25, 22, 245],
                compact_background: [24, 22, 20, 248],
                progress_track: [66, 59, 51, 255],
                progress_fill: [242, 184, 75, 255],
                divider: [72, 64, 55, 180],
                compact_dot: [116, 109, 101, 255],
                expanded_chevron: [242, 184, 75, 255],
                compact_chevron: [222, 169, 71, 255],
                primary: [246, 242, 234],
                muted: [138, 132, 125],
                accent: [242, 184, 75],
                lightning: [225, 171, 69],
                credit: [216, 168, 74],
                secondary: [198, 190, 179],
                expanded_helper: [184, 176, 165],
                expanded_status: [129, 123, 116],
            },
        }
    }
}

pub fn expanded_height_for_window_count(window_count: usize) -> i32 {
    let rows = window_count.max(1) as i32;
    if rows == 1 {
        80
    } else {
        29 + rows * 36 + (rows - 1) * 6 + 8
    }
}

fn expanded_row_layout(window_count: usize, index: usize) -> ExpandedRowLayout {
    let rows = window_count.max(1);
    let row_top = 29 + (index.min(rows - 1) as i32 * 42);
    ExpandedRowLayout {
        top: row_top,
        helper_top: row_top + 15,
        bar_top: row_top + 32,
    }
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
        theme: Theme::Dark,
        expanded,
        edge_tab: false,
        size,
        title: "Codex".to_string(),
        primary_percentage,
        status_label: status_label(&snapshot.status),
        credit_label: credit_label(credits),
        credit_value: if matches!(
            &credits.status,
            CreditStatus::Available | CreditStatus::Stale
        ) {
            credits.balance.as_deref().map(format_credit_balance)
        } else {
            None
        },
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

#[cfg(windows)]
pub fn compact_width_for_snapshot(snapshot: &UsageSnapshot, credits: &CreditSnapshot) -> i32 {
    let model = build_render_model(
        snapshot,
        credits,
        false,
        WindowSize {
            width: 1,
            height: 40,
        },
    );
    let screen_dc = unsafe { windows::Win32::Graphics::Gdi::GetDC(None) };
    if screen_dc.is_invalid() {
        return fallback_compact_layout(&model).width;
    }
    let width = measured_compact_layout(screen_dc, &model).width;
    unsafe {
        let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
    }
    width
}

#[cfg(windows)]
fn measured_compact_layout(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    model: &NativeRenderModel,
) -> CompactLayout {
    let lightning_width = measure_text_width(hdc, "\u{26a1}", 16, 600);
    let title_width = measure_text_width(hdc, &model.title, 16, 600);
    let percentage_width = measure_text_width(hdc, &model.primary_percentage, 19, 700);
    let credit_label_width = measure_text_width(hdc, "Credits", 11, 400);
    let status_width = measure_text_width(hdc, &model.status_label, 11, 400);
    let credit_value_width = model
        .credit_value
        .as_deref()
        .map(|value| measure_text_width(hdc, value, 19, 700));
    let credit_content_width = if model.credit_value.is_some() {
        0
    } else {
        measure_text_width(hdc, &model.credit_label, 11, 400)
    };
    let credit_suffix_width = if compact_credit_suffix(model).is_some() {
        measure_text_width(hdc, "· Stale", 11, 400)
    } else {
        0
    };
    compact_layout_from_widths(
        lightning_width,
        title_width,
        percentage_width,
        credit_label_width,
        credit_value_width,
        credit_suffix_width,
        credit_content_width,
        status_width,
    )
}

#[cfg(not(windows))]
pub fn compact_width_for_snapshot(snapshot: &UsageSnapshot, credits: &CreditSnapshot) -> i32 {
    let model = build_render_model(
        snapshot,
        credits,
        false,
        WindowSize {
            width: 1,
            height: 40,
        },
    );
    fallback_compact_layout(&model).width
}

fn compact_layout_from_widths(
    lightning_width: i32,
    title_width: i32,
    percentage_width: i32,
    credit_label_width: i32,
    credit_value_width: Option<i32>,
    credit_suffix_width: i32,
    credit_content_width: i32,
    status_width: i32,
) -> CompactLayout {
    let icon_left = 11;
    let icon_right = icon_left + lightning_width.max(1) + 2;
    let title_left = icon_right + 7;
    let title_right = title_left + title_width;
    let percentage_left = title_right + 8;
    let percentage_right = percentage_left + percentage_width;
    let credit_left = percentage_right + 8;
    let credit_label_right = credit_left + credit_label_width;
    let (credit_value_left, credit_value_right, credit_right) =
        if let Some(value_width) = credit_value_width {
            let value_left = credit_label_right + 3;
            let value_right = value_left + value_width;
            let suffix_gap = if credit_suffix_width > 0 { 3 } else { 0 };
            (
                Some(value_left),
                Some(value_right),
                value_right + suffix_gap + credit_suffix_width,
            )
        } else {
            (None, None, credit_left + credit_content_width)
        };
    let separator_left = credit_right + 5;
    let separator_right = separator_left + 3;
    let status_left = separator_right + 5;
    let status_right = status_left + status_width;
    let chevron_center_x = status_right + 7 + 10;
    let width = chevron_center_x + 10 + 8;

    CompactLayout {
        width,
        icon_left,
        icon_right,
        title_left,
        title_right,
        percentage_left,
        percentage_right,
        credit_left,
        credit_label_right,
        credit_value_left,
        credit_value_right,
        credit_right,
        separator_left,
        separator_right,
        status_left,
        status_right,
        chevron_center_x,
    }
}

fn fallback_compact_layout(model: &NativeRenderModel) -> CompactLayout {
    compact_layout_from_widths(
        fallback_text_width("\u{26a1}", 16),
        fallback_text_width(&model.title, 16),
        fallback_text_width(&model.primary_percentage, 19),
        fallback_text_width("Credits", 11),
        model
            .credit_value
            .as_deref()
            .map(|value| fallback_text_width(value, 19)),
        model
            .credit_value
            .as_ref()
            .map(|_| {
                if compact_credit_suffix(model).is_some() {
                    fallback_text_width("· Stale", 11)
                } else {
                    0
                }
            })
            .unwrap_or(0),
        if model.credit_value.is_some() {
            0
        } else {
            fallback_text_width(&model.credit_label, 11)
        },
        fallback_text_width(&model.status_label, 11),
    )
}

fn fallback_text_width(value: &str, size: i32) -> i32 {
    (value.chars().count() as i32 * (size / 2).max(1)).max(1)
}

fn compact_credit_suffix(model: &NativeRenderModel) -> Option<&'static str> {
    if model.credit_value.is_some() && matches!(model.credit_status, CreditStatus::Stale) {
        Some("· Stale")
    } else {
        None
    }
}

#[cfg(windows)]
fn create_segoe_font(size: i32, weight: i32) -> windows::Win32::Graphics::Gdi::HFONT {
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
        FF_DONTCARE, OUT_DEFAULT_PRECIS,
    };

    unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32,
            windows::core::w!("Segoe UI"),
        )
    }
}

#[cfg(windows)]
fn measure_text_width(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    value: &str,
    size: i32,
    weight: i32,
) -> i32 {
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetTextExtentPoint32W, SelectObject, HGDIOBJ,
    };

    let value = if value.starts_with('\u{8def}') && value.ends_with(" Stale") {
        "\u{00b7} Stale"
    } else {
        value
    };
    let font = create_segoe_font(size, weight);
    let previous = unsafe { SelectObject(hdc, HGDIOBJ(font.0)) };
    let text: Vec<u16> = value.encode_utf16().collect();
    let mut extent = SIZE::default();
    let measured = unsafe { GetTextExtentPoint32W(hdc, &text, &mut extent).as_bool() };
    unsafe {
        let _ = SelectObject(hdc, previous);
        let _ = DeleteObject(HGDIOBJ(font.0));
    }
    if measured {
        extent.cx.max(0)
    } else {
        0
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
    let (_, month, day) = civil_date_from_days(days);
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][month as usize - 1];
    format!("Reset {month_name} {day}")
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
    let compact_layout = if model.expanded {
        None
    } else {
        Some(measured_compact_layout(screen_dc, model))
    };
    draw_surface(
        pixels,
        model.size.width,
        model.size.height,
        model,
        compact_layout.as_ref(),
    );
    draw_text_surface(memory_dc, model, compact_layout.as_ref());
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
fn draw_surface(
    pixels: &mut [u32],
    width: i32,
    height: i32,
    model: &NativeRenderModel,
    compact_layout: Option<&CompactLayout>,
) {
    pixels.fill(0);
    let palette = ThemePalette::for_theme(model.theme);
    let compact = compact_layout;
    let design_width = compact.map(|layout| layout.width as f32).unwrap_or(320.0);
    let scale = width.max(1) as f32 / design_width;

    if model.edge_tab {
        let tab_width = scaled(60, scale).min(width);
        let tab_height = scaled(6, scale).min(height).max(1);
        let tab_left = (width - tab_width) / 2;
        let tab_top = height - tab_height;
        fill_rounded(
            pixels,
            width,
            height,
            tab_left,
            tab_top,
            tab_left + tab_width,
            height,
            scaled(3, scale),
            palette_rgba(palette.edge_tab),
        );
        return;
    }

    let radius = scaled(if model.expanded { 12 } else { 14 }, scale);
    let border = if model.expanded {
        palette_rgba(palette.expanded_border)
    } else {
        palette_rgba(palette.compact_border)
    };
    let background = if model.expanded {
        palette_rgba(palette.expanded_background)
    } else {
        palette_rgba(palette.compact_background)
    };
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

    if model.expanded {
        let bar_left = scaled(16, scale);
        let bar_right = width - scaled(16, scale);
        let track = palette_rgba(palette.progress_track);
        let fill = palette_rgba(palette.progress_fill);
        for (index, window) in model.windows.iter().enumerate() {
            let row = expanded_row_layout(model.windows.len(), index);
            let bar_top = scaled(row.bar_top, scale);
            let bar_bottom = bar_top + scaled(3, scale).max(1);
            fill_rounded(
                pixels,
                width,
                height,
                bar_left,
                bar_top,
                bar_right,
                bar_bottom,
                scaled(1, scale),
                track,
            );
            let filled =
                bar_left + ((bar_right - bar_left) * i32::from(window.remaining_percent) / 100);
            fill_rounded(
                pixels,
                width,
                height,
                bar_left,
                bar_top,
                filled.max(bar_left),
                bar_bottom,
                scaled(1, scale),
                fill,
            );
        }
        let divider = palette_rgba(palette.divider);
        fill_rect(
            pixels,
            width,
            height,
            scaled(16, scale),
            scaled(25, scale),
            width - scaled(16, scale),
            scaled(26, scale),
            divider,
        );
    }

    if !model.expanded {
        if let Some(layout) = compact {
            let dot_size = scaled(3, scale).max(1);
            let dot_left = scaled(layout.separator_left, scale);
            let dot_top = (height - dot_size) / 2;
            fill_rounded(
                pixels,
                width,
                height,
                dot_left,
                dot_top,
                dot_left + dot_size,
                dot_top + dot_size,
                (dot_size / 2).max(1),
                palette_rgba(palette.compact_dot),
            );
        }
    }

    draw_chevron(
        pixels,
        width,
        height,
        compact
            .map(|layout| scaled(layout.chevron_center_x, scale))
            .unwrap_or(width - scaled(16, scale)),
        if model.expanded {
            scaled(13, scale)
        } else {
            height / 2
        },
        if model.expanded {
            scaled(5, scale).max(3)
        } else {
            scaled(7, scale).max(4)
        },
        model.expanded,
        if model.expanded {
            palette_rgba(palette.expanded_chevron)
        } else {
            palette_rgba(palette.compact_chevron)
        },
    );
}

#[cfg(windows)]
fn draw_text_surface(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    model: &NativeRenderModel,
    compact_layout: Option<&CompactLayout>,
) {
    use windows::Win32::Foundation::{COLORREF, RECT};
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, DrawTextW, SelectObject, SetBkMode, SetTextColor, DT_NOPREFIX, DT_SINGLELINE,
        DT_VCENTER, FW_BOLD, FW_NORMAL, FW_SEMIBOLD, HGDIOBJ, TRANSPARENT,
    };

    if model.edge_tab {
        return;
    }

    let compact = compact_layout;
    let design_width = compact.map(|layout| layout.width as f32).unwrap_or(320.0);
    let scale = model.size.width.max(1) as f32 / design_width;
    let palette = ThemePalette::for_theme(model.theme);
    let white = palette_rgb(palette.primary);
    let muted = palette_rgb(palette.muted);
    let accent = palette_rgb(palette.accent);
    let lightning = palette_rgb(palette.lightning);
    let credit_accent = palette_rgb(palette.credit);
    let secondary = palette_rgb(palette.secondary);

    unsafe {
        let _ = SetBkMode(hdc, TRANSPARENT);
        if model.expanded {
            let expanded_title_size = scaled(16, scale);
            let expanded_credit_label_size = scaled(11, scale);
            let expanded_credit_value_size = scaled(13, scale);
            let expanded_window_size = scaled(13, scale);
            let expanded_percentage_size = scaled(18, scale);
            let expanded_helper_size = scaled(11, scale);
            let expanded_primary = white;
            let expanded_percentage = palette_rgb(palette.accent);
            let expanded_credit = palette_rgb(palette.credit);
            let expanded_helper = palette_rgb(palette.expanded_helper);
            let expanded_status = palette_rgb(palette.expanded_status);
            let content_left = scaled(16, scale);
            let content_right = model.size.width - scaled(16, scale);

            draw_text(
                hdc,
                "⚡",
                rect(
                    scaled(14, scale),
                    scaled(1, scale),
                    scaled(36, scale),
                    scaled(24, scale),
                ),
                expanded_title_size,
                expanded_percentage,
                FW_SEMIBOLD.0 as i32,
            );
            draw_text(
                hdc,
                &model.title,
                rect(
                    scaled(38, scale),
                    scaled(3, scale),
                    scaled(90, scale),
                    scaled(24, scale),
                ),
                expanded_title_size,
                expanded_primary,
                FW_SEMIBOLD.0 as i32,
            );

            let header_right = model.size.width - scaled(32, scale);
            if let Some(value) = model.credit_value.as_deref() {
                let label_width = measure_text_width(
                    hdc,
                    "Credits",
                    expanded_credit_label_size,
                    FW_NORMAL.0 as i32,
                );
                let value_width = measure_text_width(
                    hdc,
                    value,
                    expanded_credit_value_size,
                    FW_SEMIBOLD.0 as i32,
                );
                let stale_width = if matches!(model.credit_status, CreditStatus::Stale) {
                    measure_text_width(hdc, "Stale", expanded_helper_size, FW_NORMAL.0 as i32)
                } else {
                    0
                };
                let gap = scaled(4, scale);
                let stale_gap = if stale_width > 0 { gap } else { 0 };
                let group_width = label_width + gap + value_width + stale_gap + stale_width;
                let group_left = (header_right - group_width).max(scaled(100, scale));
                let label_right = group_left + label_width;
                let value_left = label_right + gap;
                let value_right = value_left + value_width;
                draw_text(
                    hdc,
                    "Credits",
                    rect(group_left, scaled(4, scale), label_right, scaled(23, scale)),
                    expanded_credit_label_size,
                    expanded_helper,
                    FW_NORMAL.0 as i32,
                );
                draw_text(
                    hdc,
                    value,
                    rect(value_left, scaled(2, scale), value_right, scaled(24, scale)),
                    expanded_credit_value_size,
                    expanded_credit,
                    FW_SEMIBOLD.0 as i32,
                );
                if stale_width > 0 {
                    draw_text(
                        hdc,
                        "Stale",
                        rect(
                            value_right + stale_gap,
                            scaled(4, scale),
                            header_right,
                            scaled(23, scale),
                        ),
                        expanded_helper_size,
                        expanded_status,
                        FW_NORMAL.0 as i32,
                    );
                }
            } else {
                let credit_width = measure_text_width(
                    hdc,
                    &model.credit_label,
                    expanded_credit_label_size,
                    FW_NORMAL.0 as i32,
                );
                let credit_left = (header_right - credit_width).max(scaled(100, scale));
                draw_text(
                    hdc,
                    &model.credit_label,
                    rect(
                        credit_left,
                        scaled(4, scale),
                        header_right,
                        scaled(23, scale),
                    ),
                    expanded_credit_label_size,
                    expanded_helper,
                    FW_NORMAL.0 as i32,
                );
            }

            if model.windows.is_empty() {
                let row = expanded_row_layout(0, 0);
                draw_text(
                    hdc,
                    "No allowance window",
                    rect(
                        content_left,
                        scaled(row.top, scale),
                        content_right,
                        scaled(row.top + 17, scale),
                    ),
                    expanded_window_size,
                    expanded_primary,
                    FW_SEMIBOLD.0 as i32,
                );
                let status_width = measure_text_width(
                    hdc,
                    &model.status_label,
                    expanded_helper_size,
                    FW_NORMAL.0 as i32,
                );
                draw_text(
                    hdc,
                    &model.status_label,
                    rect(
                        content_right - status_width,
                        scaled(row.helper_top, scale),
                        content_right,
                        scaled(row.helper_top + 14, scale),
                    ),
                    expanded_helper_size,
                    expanded_status,
                    FW_NORMAL.0 as i32,
                );
            } else {
                for (index, window) in model.windows.iter().enumerate() {
                    let row = expanded_row_layout(model.windows.len(), index);
                    let percentage_width = measure_text_width(
                        hdc,
                        &window.percentage,
                        expanded_percentage_size,
                        FW_BOLD.0 as i32,
                    );
                    let percentage_left = content_right - percentage_width;
                    draw_text(
                        hdc,
                        &window.label,
                        rect(
                            content_left,
                            scaled(row.top, scale),
                            percentage_left - scaled(10, scale),
                            scaled(row.top + 17, scale),
                        ),
                        expanded_window_size,
                        expanded_primary,
                        FW_SEMIBOLD.0 as i32,
                    );
                    draw_text(
                        hdc,
                        &window.percentage,
                        rect(
                            percentage_left,
                            scaled(row.top - 1, scale),
                            content_right,
                            scaled(row.top + 19, scale),
                        ),
                        expanded_percentage_size,
                        expanded_percentage,
                        FW_BOLD.0 as i32,
                    );
                    draw_text(
                        hdc,
                        &window.reset,
                        rect(
                            content_left,
                            scaled(row.helper_top, scale),
                            content_right,
                            scaled(row.helper_top + 14, scale),
                        ),
                        expanded_helper_size,
                        expanded_helper,
                        FW_NORMAL.0 as i32,
                    );
                    let status_width = measure_text_width(
                        hdc,
                        &model.status_label,
                        expanded_helper_size,
                        FW_NORMAL.0 as i32,
                    );
                    draw_text(
                        hdc,
                        &model.status_label,
                        rect(
                            content_right - status_width,
                            scaled(row.helper_top, scale),
                            content_right,
                            scaled(row.helper_top + 14, scale),
                        ),
                        expanded_helper_size,
                        expanded_status,
                        FW_NORMAL.0 as i32,
                    );
                }
            }
        } else {
            let collapsed_lightning_size = scaled(16, scale);
            let collapsed_title_size = scaled(16, scale);
            let collapsed_percentage_size = scaled(19, scale);
            let collapsed_credit_label_size = scaled(11, scale);
            let collapsed_credit_value_size = scaled(19, scale);
            let collapsed_status_size = scaled(11, scale);
            let layout = compact.expect("compact layout is available for collapsed rendering");
            let optical_shift = scale.round().max(1.0) as i32;
            let compact_text_rect = |left: i32, right: i32, shift: i32| {
                rect(left, shift, right, model.size.height + shift)
            };
            draw_text(
                hdc,
                "⚡",
                compact_text_rect(
                    scaled(layout.icon_left, scale),
                    scaled(layout.icon_right, scale),
                    -optical_shift,
                ),
                collapsed_lightning_size,
                lightning,
                FW_SEMIBOLD.0 as i32,
            );
            draw_text(
                hdc,
                &model.title,
                compact_text_rect(
                    scaled(layout.title_left, scale),
                    scaled(layout.title_right, scale),
                    0,
                ),
                collapsed_title_size,
                white,
                FW_SEMIBOLD.0 as i32,
            );
            draw_text(
                hdc,
                &model.primary_percentage,
                compact_text_rect(
                    scaled(layout.percentage_left, scale),
                    scaled(layout.percentage_right, scale),
                    optical_shift,
                ),
                collapsed_percentage_size,
                accent,
                FW_BOLD.0 as i32,
            );
            if let Some(value) = model.credit_value.as_deref() {
                let label_right = layout.credit_label_right;
                let value_left = layout
                    .credit_value_left
                    .expect("measured credit value left bound");
                let value_right = layout
                    .credit_value_right
                    .expect("measured credit value right bound");
                draw_text(
                    hdc,
                    "Credits",
                    compact_text_rect(
                        scaled(layout.credit_left, scale),
                        scaled(label_right, scale),
                        0,
                    ),
                    collapsed_credit_label_size,
                    secondary,
                    FW_NORMAL.0 as i32,
                );
                draw_text(
                    hdc,
                    value,
                    compact_text_rect(
                        scaled(value_left, scale),
                        scaled(value_right + COMPACT_CREDIT_VALUE_RIGHT_PADDING, scale),
                        0,
                    ),
                    collapsed_credit_value_size,
                    credit_accent,
                    FW_BOLD.0 as i32,
                );
                if matches!(model.credit_status, CreditStatus::Stale) {
                    draw_text(
                        hdc,
                        "· Stale",
                        compact_text_rect(
                            scaled(value_right + 3, scale),
                            scaled(layout.credit_right, scale),
                            0,
                        ),
                        collapsed_credit_label_size,
                        muted,
                        FW_NORMAL.0 as i32,
                    );
                }
            } else {
                draw_text(
                    hdc,
                    &model.credit_label,
                    compact_text_rect(
                        scaled(layout.credit_left, scale),
                        scaled(layout.credit_right, scale),
                        0,
                    ),
                    collapsed_credit_label_size,
                    secondary,
                    FW_NORMAL.0 as i32,
                );
            }
            draw_text(
                hdc,
                "·",
                compact_text_rect(
                    scaled(layout.separator_left, scale),
                    scaled(layout.separator_right, scale),
                    0,
                ),
                collapsed_status_size,
                palette_rgb([
                    palette.compact_dot[0],
                    palette.compact_dot[1],
                    palette.compact_dot[2],
                ]),
                FW_NORMAL.0 as i32,
            );
            draw_text(
                hdc,
                &model.status_label,
                compact_text_rect(
                    scaled(layout.status_left, scale),
                    scaled(layout.status_right, scale),
                    0,
                ),
                collapsed_status_size,
                muted,
                FW_NORMAL.0 as i32,
            );
        }
    }

    fn draw_text(
        hdc: windows::Win32::Graphics::Gdi::HDC,
        value: &str,
        mut bounds: RECT,
        size: i32,
        color: COLORREF,
        weight: i32,
    ) {
        let value = if value == "\u{8def}" {
            return;
        } else if value.chars().any(|character| character == '\u{923f}') {
            "\u{26a1}"
        } else if value.starts_with('\u{8def}') && value.ends_with(" Stale") {
            "\u{00b7} Stale"
        } else {
            value
        };
        let mut text: Vec<u16> = value.encode_utf16().collect();
        unsafe {
            let font = create_segoe_font(size, weight);
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
}

#[cfg(windows)]
fn palette_rgb(color: [u8; 3]) -> windows::Win32::Foundation::COLORREF {
    windows::Win32::Foundation::COLORREF(
        u32::from(color[0]) | (u32::from(color[1]) << 8) | (u32::from(color[2]) << 16),
    )
}

#[cfg(windows)]
fn palette_rgba(color: [u8; 4]) -> u32 {
    premultiplied_bgra(
        u32::from(color[0]),
        u32::from(color[1]),
        u32::from(color[2]),
        u32::from(color[3]),
    )
}

#[cfg(windows)]
fn scaled(value: i32, scale: f32) -> i32 {
    ((value as f32 * scale).round() as i32).max(0)
}

#[cfg(windows)]
fn draw_chevron(
    pixels: &mut [u32],
    width: i32,
    height: i32,
    center_x: i32,
    center_y: i32,
    half_width: i32,
    up: bool,
    color: u32,
) {
    for offset in 0..=half_width {
        let y = if up {
            center_y + half_width - offset
        } else {
            center_y - half_width + offset
        };
        let left_x = center_x - half_width + offset;
        let right_x = center_x + half_width - offset;
        for thickness in 0..2 {
            set_pixel(pixels, width, height, left_x, y + thickness, color);
            set_pixel(pixels, width, height, right_x, y + thickness, color);
        }
    }
}

#[cfg(windows)]
fn set_pixel(pixels: &mut [u32], width: i32, height: i32, x: i32, y: i32, color: u32) {
    if x >= 0 && x < width && y >= 0 && y < height {
        pixels[y as usize * width as usize + x as usize] = color;
    }
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
    use super::{
        build_render_model, compact_width_for_snapshot, expanded_height_for_window_count,
        expanded_row_layout, format_credit_balance,
    };
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
                width: 380,
                height: 48,
            },
        );
        assert_eq!(model.primary_percentage, "42%");
        assert_eq!(model.status_label, "Partial");
    }

    #[test]
    fn compact_width_is_content_driven_and_tighter_than_legacy_width() {
        let width = compact_width_for_snapshot(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 0)]),
            &credits(CreditStatus::Available, Some("774.86")),
        );

        assert!(width >= 240);
        assert!(width < 380);
    }

    #[cfg(windows)]
    #[test]
    fn measured_compact_layout_keeps_dynamic_content_non_overlapping() {
        use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

        let hdc = unsafe { GetDC(None) };
        assert!(!hdc.is_invalid());
        let cases = vec![
            (
                snapshot(RuntimeStatus::Partial, vec![window(10080, 0)]),
                credits(CreditStatus::Available, Some("741.74")),
            ),
            (
                snapshot(RuntimeStatus::Partial, vec![window(10080, 100)]),
                credits(CreditStatus::Available, Some("123456789012345.67")),
            ),
            (
                snapshot(RuntimeStatus::Unavailable, Vec::new()),
                credits(CreditStatus::Unavailable, None),
            ),
            (
                snapshot(RuntimeStatus::Fresh, vec![window(10080, 42)]),
                credits(CreditStatus::Available, Some("741.74")),
            ),
            (
                snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
                credits(CreditStatus::Stale, Some("841.5000")),
            ),
        ];

        for (usage, credit_snapshot) in cases {
            let model = build_render_model(
                &usage,
                &credit_snapshot,
                false,
                WindowSize {
                    width: 1,
                    height: 40,
                },
            );
            let layout = super::measured_compact_layout(hdc, &model);

            assert_eq!(layout.percentage_left - layout.title_right, 8);
            assert_eq!(layout.credit_left - layout.percentage_right, 8);
            assert_eq!(layout.separator_left - layout.credit_right, 5);
            assert_eq!(layout.separator_right - layout.separator_left, 3);
            assert_eq!(layout.status_left - layout.separator_right, 5);
            assert_eq!(layout.chevron_center_x - layout.status_right, 17);
            assert_eq!(layout.width - layout.chevron_center_x, 18);
            assert!(layout.title_left < layout.title_right);
            assert!(layout.title_right <= layout.percentage_left);
            assert!(layout.percentage_left < layout.percentage_right);
            assert!(layout.percentage_right <= layout.credit_left);
            assert!(layout.credit_left < layout.credit_right);
            assert!(layout.status_left < layout.status_right);
            assert!(layout.status_right < layout.width);

            if let (Some(value_left), Some(value_right)) =
                (layout.credit_value_left, layout.credit_value_right)
            {
                assert_eq!(value_left - layout.credit_label_right, 3);
                assert!(layout.credit_label_right <= value_left);
                assert!(value_left < value_right);
                assert!(value_right <= layout.credit_right);
            }
        }

        unsafe {
            let _ = ReleaseDC(None, hdc);
        }
    }

    #[test]
    fn unavailable_model_does_not_invent_a_percentage() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Unavailable, Vec::new()),
            &credits(CreditStatus::Unavailable, None),
            false,
            WindowSize {
                width: 380,
                height: 48,
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
                width: 320,
                height: 92,
            },
        );
        assert_eq!(model.windows.len(), 2);
        assert_eq!(model.windows[0].label, "5h");
        assert_eq!(model.windows[1].label, "1 week");
        assert_eq!(model.title, "Codex");
        assert_eq!(model.windows[0].percentage, "80%");
        assert_eq!(model.windows[1].percentage, "42%");
    }

    #[test]
    fn expanded_height_grows_only_for_additional_allowance_rows() {
        assert_eq!(expanded_height_for_window_count(0), 80);
        assert_eq!(expanded_height_for_window_count(1), 80);
        assert_eq!(expanded_height_for_window_count(2), 115);
        assert_eq!(expanded_height_for_window_count(3), 157);
    }

    #[test]
    fn expanded_rows_have_separate_vertical_regions() {
        let first = expanded_row_layout(2, 0);
        let second = expanded_row_layout(2, 1);

        assert_eq!(first.top, 29);
        assert_eq!(first.helper_top, 44);
        assert_eq!(first.bar_top, 61);
        assert!(first.bar_top + 3 < second.top);
        assert_eq!(second.top, 71);
    }

    #[test]
    fn stale_model_keeps_the_last_valid_window_value() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Stale, vec![window(10080, 0)]),
            &credits(CreditStatus::Unavailable, None),
            false,
            WindowSize {
                width: 380,
                height: 48,
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
                width: 380,
                height: 48,
            },
        );
        assert_eq!(model.credit_label, "Credits 827.96");
        assert_eq!(model.credit_value.as_deref(), Some("827.96"));
    }

    #[test]
    fn expanded_model_renders_compact_credit_row() {
        let model = build_render_model(
            &snapshot(RuntimeStatus::Partial, vec![window(10080, 42)]),
            &credits(CreditStatus::Available, Some("841.5000")),
            true,
            WindowSize {
                width: 320,
                height: 92,
            },
        );
        assert_eq!(model.credit_label, "Credits 841.5");
        assert_eq!(
            model.size,
            WindowSize {
                width: 320,
                height: 92
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
                width: 380,
                height: 48,
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
                    width: 380,
                    height: 48,
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
                width: 320,
                height: 92,
            },
        );
        assert_eq!(model.credit_label, "Credits 841 · Stale");
        assert_eq!(model.credit_value.as_deref(), Some("841"));
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
                width: 380,
                height: 48,
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

#[cfg(all(test, windows))]
mod theme_tests {
    use super::ThemePalette;
    use crate::settings::Theme;

    #[test]
    fn dark_palette_preserves_the_phase_6_surface_and_text_colors() {
        let palette = ThemePalette::for_theme(Theme::Dark);

        assert_eq!(palette.expanded_background, [28, 25, 22, 245]);
        assert_eq!(palette.compact_background, [24, 22, 20, 248]);
        assert_eq!(palette.primary, [246, 242, 234]);
        assert_eq!(palette.accent, [242, 184, 75]);
        assert_eq!(palette.credit, [216, 168, 74]);
    }

    #[test]
    fn light_palette_changes_colors_without_changing_palette_roles() {
        let dark = ThemePalette::for_theme(Theme::Dark);
        let light = ThemePalette::for_theme(Theme::Light);

        assert_ne!(light.expanded_background, dark.expanded_background);
        assert_ne!(light.compact_background, dark.compact_background);
        assert_ne!(light.primary, dark.primary);
        assert_eq!(light.progress_fill.len(), dark.progress_fill.len());
        assert_eq!(light.primary.len(), dark.primary.len());
    }
}
