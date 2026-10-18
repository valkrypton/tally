//! The Night shift look from `DESIGN.md`: colour tokens, fonts, text helpers and the
//! style functions every view uses. Views never write colours or sizes directly.

use iced::border::Radius;
use iced::font::Weight;
use iced::widget::{Text, button, checkbox, container, radio, scrollable, svg, text, text_input};
use iced::{Background, Border, Color, Font, Theme};

// ---- Colour tokens (DESIGN.md front matter) ----
pub const LAMP_AMBER: Color = Color::from_rgb8(0xFF, 0xB2, 0x3E);
pub const LAMP_AMBER_HOVER: Color = Color::from_rgb8(0xFF, 0xC8, 0x6E);
pub const LAMP_AMBER_WASH: Color = Color::from_rgb8(0x24, 0x1E, 0x14);
pub const LAMP_AMBER_INK: Color = Color::from_rgb8(0xF5, 0xE6, 0xCC);
pub const LAMP_AMBER_INK_MUTED: Color = Color::from_rgb8(0xC9, 0xB4, 0x8F);
/// The disabled primary button fill, taken from the B-AddTime artboard.
pub const LAMP_AMBER_DISABLED: Color = Color::from_rgb8(0x2E, 0x27, 0x1B);
pub const CONSOLE_NIGHT: Color = Color::from_rgb8(0x12, 0x14, 0x17);
pub const CONSOLE_DEEP: Color = Color::from_rgb8(0x0E, 0x10, 0x12);
pub const CONSOLE_RAISED: Color = Color::from_rgb8(0x1A, 0x1D, 0x21);
pub const CONSOLE_HOVER: Color = Color::from_rgb8(0x1E, 0x22, 0x26);
pub const CONSOLE_FLOATING: Color = Color::from_rgb8(0x22, 0x26, 0x2B);
pub const CONSOLE_TRACK: Color = Color::from_rgb8(0x23, 0x27, 0x2C);
pub const CONSOLE_SELECTED: Color = Color::from_rgb8(0x2C, 0x31, 0x37);
pub const HAIRLINE: Color = Color::from_rgb8(0x26, 0x2A, 0x2F);
pub const OUTLINE: Color = Color::from_rgb8(0x34, 0x3A, 0x40);
pub const TEXT_PRIMARY: Color = Color::from_rgb8(0xE8, 0xEA, 0xED);
pub const TEXT_MUTED: Color = Color::from_rgb8(0xA3, 0xAA, 0xB2);
pub const TEXT_PLACEHOLDER: Color = Color::from_rgb8(0x8A, 0x91, 0x99);
pub const TEXT_DISABLED: Color = Color::from_rgb8(0x4A, 0x50, 0x58);
pub const STATUS_TRACKING: Color = Color::from_rgb8(0x7C, 0xE3, 0x8B);
pub const ALERT: Color = Color::from_rgb8(0xFF, 0x8A, 0x80);
pub const ALERT_WASH: Color = Color::from_rgb8(0x2B, 0x1A, 0x1A);
/// The faint dark tint behind a key cap sitting on an amber button: `rgba(18,20,23,0.14)`.
pub const KEY_ON_AMBER: Color = Color::from_rgba8(0x12, 0x14, 0x17, 0.14);

// ---- Radii ----
pub const R_BAR: f32 = 3.0;
pub const R_KEY: f32 = 4.0;
pub const R_CONTROL: f32 = 6.0;
pub const R_FIELD: f32 = 8.0;
pub const R_MENU: f32 = 10.0;
pub const R_POPUP: f32 = 14.0;

// ---- Spacing ----
pub const S_HAIR: f32 = 2.0;
pub const S_XS: f32 = 4.0;
pub const S_SM: f32 = 8.0;
pub const S_MD: f32 = 12.0;
pub const S_LG: f32 = 16.0;
pub const S_XL: f32 = 20.0;
pub const S_XXL: f32 = 24.0;

// ---- Fonts (bundled in `assets/fonts`, loaded in `main.rs`) ----
pub const UI_FAMILY: &str = "Schibsted Grotesk";
pub const MONO_FAMILY: &str = "JetBrains Mono";
pub const UI: Font = Font::with_name(UI_FAMILY);
pub const UI_MEDIUM: Font = weighted(UI, Weight::Medium);
pub const UI_SEMIBOLD: Font = weighted(UI, Weight::Semibold);
pub const UI_BOLD: Font = weighted(UI, Weight::Bold);
pub const MONO: Font = Font::with_name(MONO_FAMILY);
pub const MONO_MEDIUM: Font = weighted(MONO, Weight::Medium);
pub const MONO_BOLD: Font = weighted(MONO, Weight::Bold);

const fn weighted(font: Font, weight: Weight) -> Font {
    Font {
        family: font.family,
        weight,
        stretch: font.stretch,
        style: font.style,
    }
}

/// Every font file bundled into the binary.
pub const FONT_FILES: [&[u8]; 7] = [
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Regular.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Medium.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/SchibstedGrotesk-Bold.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Medium.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
];

/// The iced theme. Only its palette is used, by widgets we don't restyle (scrollbars).
pub fn theme() -> Theme {
    Theme::custom(
        String::from("Night shift"),
        iced::theme::Palette {
            background: CONSOLE_NIGHT,
            text: TEXT_PRIMARY,
            primary: LAMP_AMBER,
            success: STATUS_TRACKING,
            warning: LAMP_AMBER,
            danger: ALERT,
        },
    )
}

/// `⌘` on macOS, `Ctrl` elsewhere, for key caps.
pub fn command_key() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl "
    }
}

// ---- Text helpers (DESIGN.md typography) ----

/// Headline: main-window section titles. 700, 20px.
pub fn headline<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(UI_BOLD).size(20).color(TEXT_PRIMARY)
}

/// Title: popup questions. 600, 15px.
pub fn title<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(UI_SEMIBOLD).size(15).color(TEXT_PRIMARY)
}

/// Body: names, menu items, field text. 400, 14px.
pub fn body<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(UI).size(14).color(TEXT_PRIMARY)
}

/// Body small: button labels, notes. 400, 13px.
pub fn small<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(UI).size(13).color(TEXT_PRIMARY)
}

/// Label: secondary details. 400, 12px, muted.
pub fn label<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(UI).size(12).color(TEXT_MUTED)
}

/// Figure: durations, times, dates. Mono 13px.
pub fn figure<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(MONO).size(13).color(TEXT_PRIMARY)
}

/// Figure display: the dashboard total. Mono 700, 28px, amber.
pub fn figure_display<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(MONO_BOLD).size(28).color(LAMP_AMBER)
}

/// Key: key-cap labels. Mono 11px (the only 11px text).
pub fn key<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).font(MONO).size(11).color(TEXT_MUTED)
}

// ---- Containers ----

fn border(color: Color, width: f32, radius: f32) -> Border {
    Border {
        color,
        width,
        radius: Radius::from(radius),
    }
}

fn filled(color: Color) -> Option<Background> {
    Some(Background::Color(color))
}

/// The outer frame of a popup window: night surface, hairline edge, 14px corners.
pub fn popup_frame(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_NIGHT),
        border: border(HAIRLINE, 1.0, R_POPUP),
        text_color: Some(TEXT_PRIMARY),
        ..container::Style::default()
    }
}

/// The main window's content area.
pub fn main_surface(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_NIGHT),
        text_color: Some(TEXT_PRIMARY),
        ..container::Style::default()
    }
}

/// The main window sidebar and the export preview: one step darker.
pub fn deep_surface(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_DEEP),
        ..container::Style::default()
    }
}

/// The export preview block: deep surface, hairline border, 8px corners.
pub fn preview_block(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_DEEP),
        border: border(HAIRLINE, 1.0, R_FIELD),
        ..container::Style::default()
    }
}

/// A 1px hairline divider (use inside a container of height 1).
pub fn hairline(_: &Theme) -> container::Style {
    container::Style {
        background: filled(HAIRLINE),
        ..container::Style::default()
    }
}

/// A 1px outline-coloured divider inside floating menus.
pub fn menu_divider(_: &Theme) -> container::Style {
    container::Style {
        background: filled(OUTLINE),
        ..container::Style::default()
    }
}

/// Menus that float over content: floating surface, outline border, 10px corners.
pub fn floating_menu(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_FLOATING),
        border: border(OUTLINE, 1.0, R_MENU),
        ..container::Style::default()
    }
}

/// Dims content under an open menu (the artboards show it at 40% opacity, which is
/// the night colour laid over it at 60%).
pub fn scrim(_: &Theme) -> container::Style {
    container::Style {
        background: filled(Color {
            a: 0.6,
            ..CONSOLE_NIGHT
        }),
        ..container::Style::default()
    }
}

/// Raised panel (edit-in-place, add row): raised fill, 8px corners.
pub fn raised_panel(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_RAISED),
        border: border(Color::TRANSPARENT, 0.0, R_FIELD),
        ..container::Style::default()
    }
}

/// Raised panel being answered or edited: amber outline.
pub fn raised_panel_active(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_RAISED),
        border: border(LAMP_AMBER, 1.0, R_FIELD),
        ..container::Style::default()
    }
}

/// The amber wash behind the late-answer notice.
pub fn notice_wash(_: &Theme) -> container::Style {
    container::Style {
        background: filled(LAMP_AMBER_WASH),
        border: border(Color::TRANSPARENT, 0.0, R_FIELD),
        ..container::Style::default()
    }
}

/// Error notice: alert wash with alert text.
pub fn alert_notice(_: &Theme) -> container::Style {
    container::Style {
        background: filled(ALERT_WASH),
        border: border(Color::TRANSPARENT, 0.0, R_FIELD),
        text_color: Some(ALERT),
        ..container::Style::default()
    }
}

/// The 36px icon tile in the data-file error window.
pub fn alert_tile(_: &Theme) -> container::Style {
    container::Style {
        background: filled(ALERT_WASH),
        border: border(Color::TRANSPARENT, 0.0, R_FIELD),
        ..container::Style::default()
    }
}

/// A list row: selected rows take Console Hover.
pub fn list_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: selected.then_some(Background::Color(CONSOLE_HOVER)),
        border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
        ..container::Style::default()
    }
}

/// The 20px number chip in a prompt row: amber when selected, outlined otherwise.
pub fn number_chip(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        if selected {
            container::Style {
                background: filled(LAMP_AMBER),
                border: border(Color::TRANSPARENT, 0.0, R_KEY),
                text_color: Some(CONSOLE_NIGHT),
                ..container::Style::default()
            }
        } else {
            container::Style {
                border: border(OUTLINE, 1.0, R_KEY),
                text_color: Some(TEXT_MUTED),
                ..container::Style::default()
            }
        }
    }
}

/// A key cap such as `esc` after a ghost button label.
pub fn key_cap(_: &Theme) -> container::Style {
    container::Style {
        border: border(OUTLINE, 1.0, R_KEY),
        ..container::Style::default()
    }
}

/// A key cap on an amber button: faint dark tint, no border.
pub fn key_cap_on_amber(_: &Theme) -> container::Style {
    container::Style {
        background: filled(KEY_ON_AMBER),
        border: border(Color::TRANSPARENT, 0.0, R_KEY),
        ..container::Style::default()
    }
}

/// The small "archived" tag in the Activities list.
pub fn tag(_: &Theme) -> container::Style {
    container::Style {
        border: border(OUTLINE, 1.0, R_KEY),
        ..container::Style::default()
    }
}

/// The segmented control track.
pub fn segment_track(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_RAISED),
        border: border(Color::TRANSPARENT, 0.0, R_FIELD),
        ..container::Style::default()
    }
}

/// The empty part of a bar.
pub fn bar_track(_: &Theme) -> container::Style {
    container::Style {
        background: filled(CONSOLE_TRACK),
        border: border(Color::TRANSPARENT, 0.0, R_BAR),
        ..container::Style::default()
    }
}

/// The filled part of a bar: amber for categories, muted for activities.
pub fn bar_fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: filled(color),
        border: border(Color::TRANSPARENT, 0.0, R_BAR),
        ..container::Style::default()
    }
}

/// A small round status dot (8px): green when tracking, disabled grey otherwise.
pub fn status_dot(tracking: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: filled(if tracking {
            STATUS_TRACKING
        } else {
            TEXT_DISABLED
        }),
        border: border(Color::TRANSPARENT, 0.0, 4.0),
        ..container::Style::default()
    }
}

/// The 8px amber square next to the "tally" wordmark.
pub fn brand_mark(_: &Theme) -> container::Style {
    container::Style {
        background: filled(LAMP_AMBER),
        border: border(Color::TRANSPARENT, 0.0, 2.0),
        ..container::Style::default()
    }
}

// ---- Buttons ----

/// Primary: amber fill, night text. Raised amber on hover; dark wash when disabled.
pub fn button_primary(_: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (LAMP_AMBER_HOVER, CONSOLE_NIGHT),
        button::Status::Disabled => (LAMP_AMBER_DISABLED, LAMP_AMBER_INK_MUTED),
        button::Status::Active => (LAMP_AMBER, CONSOLE_NIGHT),
    };
    button::Style {
        background: filled(background),
        text_color,
        border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
        ..button::Style::default()
    }
}

/// Ghost: no fill, primary text, Console Hover on hover.
pub fn button_ghost(_: &Theme, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (filled(CONSOLE_HOVER), TEXT_PRIMARY),
        button::Status::Disabled => (None, TEXT_DISABLED),
        button::Status::Active => (None, TEXT_PRIMARY),
    };
    button::Style {
        background,
        text_color,
        border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
        ..button::Style::default()
    }
}

/// Ghost with an outline border ("Open folder").
pub fn button_outlined(theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        border: border(OUTLINE, 1.0, R_CONTROL),
        ..button_ghost(theme, status)
    }
}

/// A ghost button that is currently "on" (the More button while its menu is open).
pub fn button_ghost_on(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: filled(HAIRLINE),
        text_color: TEXT_PRIMARY,
        border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
        ..button::Style::default()
    }
}

/// A prompt row: Console Hover when selected, Console Raised on hover.
pub fn row_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if selected {
            filled(CONSOLE_HOVER)
        } else if hovered {
            filled(CONSOLE_RAISED)
        } else {
            None
        };
        button::Style {
            background,
            text_color: TEXT_PRIMARY,
            border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
            ..button::Style::default()
        }
    }
}

/// A duration shown as a button so it can be clicked to edit.
pub fn button_total(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => filled(CONSOLE_HOVER),
        _ => None,
    };
    button::Style {
        background,
        text_color: TEXT_PRIMARY,
        border: border(Color::TRANSPARENT, 0.0, R_KEY),
        ..button::Style::default()
    }
}

/// A segmented-control option.
pub fn segment(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (background, text_color) = if selected {
            (filled(CONSOLE_SELECTED), TEXT_PRIMARY)
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => (None, TEXT_PRIMARY),
                _ => (None, TEXT_MUTED),
            }
        };
        button::Style {
            background,
            text_color,
            border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
            ..button::Style::default()
        }
    }
}

/// A tracked block in the calendar: the category colour at 20% fill, 65% edge.
pub fn calendar_block(color: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: filled(color.scale_alpha(if hovered { 0.30 } else { 0.20 })),
            text_color: TEXT_PRIMARY,
            border: border(color.scale_alpha(0.65), 1.0, R_KEY),
            ..button::Style::default()
        }
    }
}

/// An untracked working-hours gap. iced borders are solid, so no dashes.
pub fn calendar_gap(_: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hovered.then(|| Background::Color(LAMP_AMBER.scale_alpha(0.08))),
        text_color: LAMP_AMBER_INK_MUTED,
        border: border(LAMP_AMBER.scale_alpha(0.45), 1.0, R_KEY),
        ..button::Style::default()
    }
}

fn day_surface(working: bool, today: bool) -> (Color, Border) {
    let background = if working {
        Color::from_rgb8(0x16, 0x19, 0x1C)
    } else {
        CONSOLE_DEEP
    };
    let edge = if today {
        LAMP_AMBER.scale_alpha(0.45)
    } else {
        Color::from_rgb8(0x1C, 0x1F, 0x23)
    };
    (background, border(edge, 1.0, R_CONTROL))
}

/// One day's column in the week and day timelines.
pub fn calendar_day(working: bool, today: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| {
        let (background, border) = day_surface(working, today);
        container::Style {
            background: filled(background),
            border,
            ..container::Style::default()
        }
    }
}

/// One day's cell in the month grid.
pub fn month_cell(
    in_month: bool,
    working: bool,
    today: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let (background, border) = day_surface(working, today);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: filled(if hovered { CONSOLE_HOVER } else { background }),
            text_color: if in_month {
                TEXT_PRIMARY
            } else {
                TEXT_DISABLED
            },
            border,
            ..button::Style::default()
        }
    }
}

/// An option button in the prompt's More grid. `focused` is the keyboard focus: amber
/// wash and outline, like a selected chip.
pub fn menu_option(focused: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (background, text_color, border_color) = if focused {
            (LAMP_AMBER_WASH, LAMP_AMBER_INK, LAMP_AMBER)
        } else if hovered {
            (OUTLINE, TEXT_PRIMARY, Color::TRANSPARENT)
        } else {
            (CONSOLE_SELECTED, TEXT_PRIMARY, Color::TRANSPARENT)
        };
        button::Style {
            background: filled(background),
            text_color,
            border: border(border_color, 1.0, R_CONTROL),
            ..button::Style::default()
        }
    }
}

/// An item in a vertical menu: transparent until hovered or current.
pub fn menu_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (selected || hovered).then_some(Background::Color(CONSOLE_SELECTED)),
            text_color: TEXT_PRIMARY,
            border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
            ..button::Style::default()
        }
    }
}

/// A pill chip in the away prompt. `dashed` is the "Don't count it" chip, which iced
/// can't dash, so it uses the disabled-grey border and muted text instead.
pub fn chip(selected: bool, dashed: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (background, border_color, text_color) = if selected {
            (filled(LAMP_AMBER_WASH), LAMP_AMBER, LAMP_AMBER_INK)
        } else if dashed {
            (
                hovered.then_some(Background::Color(CONSOLE_HOVER)),
                TEXT_DISABLED,
                TEXT_MUTED,
            )
        } else {
            (
                hovered.then_some(Background::Color(CONSOLE_HOVER)),
                OUTLINE,
                TEXT_PRIMARY,
            )
        };
        button::Style {
            background,
            text_color,
            border: border(border_color, 1.0, 16.0),
            ..button::Style::default()
        }
    }
}

/// A sidebar navigation item.
pub fn nav_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: selected.then_some(Background::Color(CONSOLE_HOVER)),
            text_color: if selected || hovered {
                TEXT_PRIMARY
            } else {
                TEXT_MUTED
            },
            border: border(Color::TRANSPARENT, 0.0, R_CONTROL),
            ..button::Style::default()
        }
    }
}

/// The −/+ halves of a stepper.
pub fn stepper_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => CONSOLE_HOVER,
        _ => CONSOLE_RAISED,
    };
    button::Style {
        background: filled(background),
        text_color: if status == button::Status::Disabled {
            TEXT_DISABLED
        } else {
            TEXT_PRIMARY
        },
        border: border(Color::TRANSPARENT, 0.0, 0.0),
        ..button::Style::default()
    }
}

/// The stepper's outer border.
pub fn stepper_frame(_: &Theme) -> container::Style {
    container::Style {
        border: border(OUTLINE, 1.0, R_CONTROL),
        ..container::Style::default()
    }
}

/// The shortcut button in Settings: raised fill, outline border; amber while recording.
pub fn shortcut_button(recording: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: filled(CONSOLE_RAISED),
        text_color: TEXT_PRIMARY,
        border: border(if recording { LAMP_AMBER } else { OUTLINE }, 1.0, R_CONTROL),
        ..button::Style::default()
    }
}

/// A key cap inside the shortcut button.
pub fn shortcut_cap(_: &Theme) -> container::Style {
    container::Style {
        border: border(TEXT_DISABLED, 1.0, R_KEY),
        ..container::Style::default()
    }
}

// ---- Fields ----

/// How a text field should look beyond its focus state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldLook {
    /// Outline border; amber while focused.
    Resting,
    /// Always amber: the field being answered or edited.
    Active,
    /// Alert border: invalid input.
    Error,
}

/// A text field on a raised surface (prompt, search, settings).
pub fn field(look: FieldLook) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    field_on(look, CONSOLE_RAISED)
}

/// A text field inside a raised panel, which sits on the night colour instead.
pub fn inset_field(look: FieldLook) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    field_on(look, CONSOLE_NIGHT)
}

fn field_on(
    look: FieldLook,
    fill: Color,
) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let focused = matches!(status, text_input::Status::Focused { .. });
        let disabled = status == text_input::Status::Disabled;
        let border_color = match look {
            FieldLook::Error => ALERT,
            _ if disabled => CONSOLE_TRACK,
            FieldLook::Active => LAMP_AMBER,
            FieldLook::Resting if focused => LAMP_AMBER,
            FieldLook::Resting => OUTLINE,
        };
        text_input::Style {
            background: Background::Color(if disabled { Color::TRANSPARENT } else { fill }),
            border: border(border_color, 1.0, R_CONTROL),
            icon: TEXT_MUTED,
            placeholder: TEXT_PLACEHOLDER,
            value: if disabled {
                TEXT_DISABLED
            } else {
                TEXT_PRIMARY
            },
            selection: Color {
                a: 0.35,
                ..LAMP_AMBER
            },
        }
    }
}

/// The prompt's 48px answer field: 8px corners.
pub fn prompt_field(look: FieldLook) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |theme, status| text_input::Style {
        border: Border {
            radius: Radius::from(R_FIELD),
            ..field(look)(theme, status).border
        },
        ..field(look)(theme, status)
    }
}

// ---- Toggles ----

/// Amber checkbox.
pub fn checkbox_style(_: &Theme, status: checkbox::Status) -> checkbox::Style {
    let checked = match status {
        checkbox::Status::Active { is_checked }
        | checkbox::Status::Hovered { is_checked }
        | checkbox::Status::Disabled { is_checked } => is_checked,
    };
    checkbox::Style {
        background: Background::Color(if checked { LAMP_AMBER } else { CONSOLE_RAISED }),
        icon_color: CONSOLE_NIGHT,
        border: border(if checked { LAMP_AMBER } else { OUTLINE }, 1.0, R_KEY),
        text_color: Some(TEXT_PRIMARY),
    }
}

/// Amber radio button.
pub fn radio_style(_: &Theme, status: radio::Status) -> radio::Style {
    let selected = match status {
        radio::Status::Active { is_selected } | radio::Status::Hovered { is_selected } => {
            is_selected
        }
    };
    radio::Style {
        background: Background::Color(CONSOLE_RAISED),
        dot_color: LAMP_AMBER,
        border_width: 1.0,
        border_color: if selected { LAMP_AMBER } else { OUTLINE },
        text_color: Some(TEXT_PRIMARY),
    }
}

/// Scrollbars: a thin track-coloured scroller on no rail.
pub fn scroll(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    let rail = scrollable::Rail {
        background: None,
        border: border(Color::TRANSPARENT, 0.0, 0.0),
        scroller: scrollable::Scroller {
            background: Background::Color(CONSOLE_SELECTED),
            border: border(Color::TRANSPARENT, 0.0, R_BAR),
        },
    };
    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style
}

/// Tints a monochrome SVG icon.
pub fn icon_color(color: Color) -> impl Fn(&Theme, svg::Status) -> svg::Style {
    move |_, _| svg::Style { color: Some(color) }
}

#[cfg(test)]
mod tests;
