//! Building blocks shared by every window: key caps, footer buttons, the segmented
//! control, bars, the status line and icons. Each one maps to a DESIGN.md component.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, svg, text};
use iced::{Color, Element, Length, Padding};
use tally_core::range::{RangeSpec, RangeUnit};

use crate::theme::{self, FieldLook};

/// A tinted icon at `size` px.
pub fn icon<'a, M: 'a>(handle: svg::Handle, size: f32, color: Color) -> Element<'a, M> {
    svg(handle)
        .width(size)
        .height(size)
        .style(theme::icon_color(color))
        .into()
}

/// A key cap such as `esc`, outlined.
pub fn key_cap<'a, M: 'a>(label: impl text::IntoFragment<'a>) -> Element<'a, M> {
    container(theme::key(label))
        .padding(Padding::from([1.0, 5.0]))
        .style(theme::key_cap)
        .into()
}

/// A key cap inside an amber button.
pub fn key_cap_on_amber<'a, M: 'a>(label: impl text::IntoFragment<'a>) -> Element<'a, M> {
    container(
        theme::key(label)
            .font(theme::MONO_MEDIUM)
            .color(theme::CONSOLE_NIGHT),
    )
    .padding(Padding::from([1.0, 5.0]))
    .style(theme::key_cap_on_amber)
    .into()
}

/// A 40px ghost footer button with its key cap after the label.
pub fn ghost_button<'a, M: Clone + 'a>(
    label: &'a str,
    key: Option<&'a str>,
    on_press: Option<M>,
) -> Element<'a, M> {
    let mut content = row![theme::small(label)]
        .spacing(theme::S_SM)
        .align_y(Vertical::Center);
    if let Some(key) = key {
        content = content.push(key_cap(key));
    }
    button(
        container(content)
            .height(Length::Fill)
            .align_y(Vertical::Center),
    )
    .height(40)
    .padding(Padding::from([0.0, 10.0]))
    .style(theme::button_ghost)
    .on_press_maybe(on_press)
    .into()
}

/// The one amber primary action: optional leading icon, label, optional key cap.
/// `None` for `on_press` draws it disabled.
pub fn primary_button<'a, M: Clone + 'a>(
    label: String,
    leading: Option<svg::Handle>,
    key: Option<&'a str>,
    on_press: Option<M>,
) -> Element<'a, M> {
    let enabled = on_press.is_some();
    let ink = if enabled {
        theme::CONSOLE_NIGHT
    } else {
        theme::LAMP_AMBER_INK_MUTED
    };
    let has_icon = leading.is_some();
    let mut content = row![].spacing(theme::S_SM).align_y(Vertical::Center);
    if let Some(handle) = leading {
        content = content.push(icon(handle, 14.0, ink));
    }
    content = content.push(text(label).font(theme::UI_SEMIBOLD).size(14));
    let right_padding = if key.is_some() { 8.0 } else { 14.0 };
    if let Some(key) = key {
        content = content.push(key_cap_on_amber(key));
    }
    button(
        container(content)
            .height(Length::Fill)
            .align_y(Vertical::Center),
    )
    .height(40)
    .padding(Padding {
        top: 0.0,
        right: right_padding,
        bottom: 0.0,
        left: if has_icon { 10.0 } else { 14.0 },
    })
    .style(theme::button_primary)
    .on_press_maybe(on_press)
    .into()
}

/// A square 40px ghost icon button (pause, menu, arrows).
pub fn icon_button<'a, M: Clone + 'a>(
    handle: svg::Handle,
    size: f32,
    on_press: Option<M>,
) -> Element<'a, M> {
    let color = if on_press.is_some() {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_DISABLED
    };
    button(container(icon(handle, size, color)).center(Length::Fill))
        .width(40)
        .height(40)
        .padding(0)
        .style(theme::button_ghost)
        .on_press_maybe(on_press)
        .into()
}

/// The footer bar every popup ends with: a hairline, then 8px-padded actions.
pub fn footer<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![divider(), container(content).padding(theme::S_SM)].into()
}

/// A ‹ or › button, disabled when the range can't move that way.
pub fn step_arrow<'a, M: Clone + 'a>(
    handle: svg::Handle,
    spec: RangeSpec,
    steps: i32,
    today: jiff::civil::Date,
    on_step: impl Fn(i32) -> M,
) -> Element<'a, M> {
    let enabled = super::dashboard::step_within(spec, steps, today).is_some();
    icon_button(handle, 16.0, enabled.then(|| on_step(steps)))
}

/// The hint under an activity rename: the merge offer, the error, or what Save does.
pub fn rename_hint<'a>(merging: bool, invalid: bool) -> text::Text<'a> {
    if merging {
        theme::label("That name is taken. Enter merges into it").color(theme::LAMP_AMBER)
    } else if invalid {
        theme::label("Type a name, or Category: Name").color(theme::ALERT)
    } else {
        theme::label("Renames it everywhere")
    }
}

/// A full-width 1px hairline.
pub fn divider<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new())
        .width(Length::Fill)
        .height(1)
        .style(theme::hairline)
        .into()
}

pub const UNITS: [(RangeUnit, &str); 5] = [
    (RangeUnit::Day, "Day"),
    (RangeUnit::Week, "Week"),
    (RangeUnit::Month, "Month"),
    (RangeUnit::Year, "Year"),
    (RangeUnit::All, "All"),
];

/// Day / Week / Month / Year / All. `fill` stretches the options to equal widths.
pub fn unit_picker<'a, M: Clone + 'a>(
    current: RangeUnit,
    fill: bool,
    on_pick: impl Fn(RangeUnit) -> M,
) -> Element<'a, M> {
    let options = UNITS.iter().map(|(unit, name)| {
        let selected = *unit == current;
        let label = text(*name)
            .size(12)
            .font(if selected {
                theme::UI_SEMIBOLD
            } else {
                theme::UI
            })
            .width(if fill { Length::Fill } else { Length::Shrink })
            .center();
        button(label)
            .height(30)
            .width(if fill { Length::Fill } else { Length::Shrink })
            .padding(Padding::from([0.0, 12.0]))
            .style(theme::segment(selected))
            .on_press(on_pick(*unit))
            .into()
    });
    container(row(options).spacing(theme::S_HAIR))
        .padding(3)
        .style(theme::raised_panel)
        .into()
}

/// The parts of a bar out of 1000: (filled, empty).
pub fn bar_portions(value: i64, max: i64) -> (u16, u16) {
    if max <= 0 || value <= 0 {
        return (0, 1000);
    }
    let filled = ((value.min(max) as f64 / max as f64) * 1000.0).round() as u16;
    (filled.max(1), 1000 - filled.max(1))
}

/// A proportion bar drawn as two containers (never as text characters).
pub fn bar<'a, M: 'a>(value: i64, max: i64, thickness: f32, fill: Color) -> Element<'a, M> {
    let (filled, empty) = bar_portions(value, max);
    let mut inner = row![].height(thickness);
    if filled > 0 {
        inner = inner.push(
            container(Space::new())
                .width(Length::FillPortion(filled))
                .height(thickness)
                .style(theme::bar_fill(fill)),
        );
    }
    if empty > 0 {
        inner = inner.push(Space::new().width(Length::FillPortion(empty)));
    }
    container(inner)
        .width(Length::Fill)
        .height(thickness)
        .style(theme::bar_fill(theme::CONSOLE_TRACK))
        .into()
}

/// The status line: a dot, a sentence, and an optional figure such as `16:45`.
pub fn status_line<'a, M: 'a>(status: &crate::status::Status) -> Element<'a, M> {
    let dot = container(Space::new())
        .width(8)
        .height(8)
        .style(theme::status_dot(status.tracking));
    let mut line = row![dot, theme::label(status.text.clone())]
        .spacing(theme::S_SM)
        .align_y(Vertical::Center);
    if let Some(figure) = &status.figure {
        line = line.push(
            text(figure.clone())
                .font(theme::MONO)
                .size(12)
                .color(theme::TEXT_PRIMARY),
        );
    }
    line.into()
}

/// A labelled −/+ stepper: `15 min`.
pub fn stepper<'a, M: Clone + 'a>(
    value: String,
    on_minus: Option<M>,
    on_plus: Option<M>,
) -> Element<'a, M> {
    let half = |handle, on_press: Option<M>| {
        let color = if on_press.is_some() {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_DISABLED
        };
        button(container(icon(handle, 12.0, color)).center(Length::Fill))
            .width(32)
            .height(32)
            .padding(0)
            .style(theme::stepper_button)
            .on_press_maybe(on_press)
    };
    container(
        row![
            half(crate::icons::minus(), on_minus),
            theme::figure(value).width(64).center(),
            half(crate::icons::plus(), on_plus),
        ]
        .align_y(Vertical::Center),
    )
    .padding(1)
    .style(theme::stepper_frame)
    .into()
}

/// The look of a field given whether its content is invalid.
pub fn look(invalid: bool, active: bool) -> FieldLook {
    if invalid {
        FieldLook::Error
    } else if active {
        FieldLook::Active
    } else {
        FieldLook::Resting
    }
}

#[cfg(test)]
mod tests;
