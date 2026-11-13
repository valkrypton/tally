//! The Calendar section: Day/Week/Month, stepping, and the editor that renames a
//! block or logs time into a gap.

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{
    Id, Space, button, column, container, row, scrollable, stack, text, text_input,
};
use iced::{Element, Length, Padding};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use tally_core::ActivityName;
use tally_core::range::{RangeSpec, RangeUnit};
use tally_core::settings::WeeklySchedule;
use tally_core::store::{ActivityId, EntryRow, Totals};

use crate::windows::KeyPress;
use crate::windows::dashboard::{self, LineKind, step_within};
use crate::windows::widgets::{self, divider, ghost_button, primary_button, step_arrow};
use crate::{format, icons, theme};
use layout::{Block, Slot, category_color, portion};

pub mod layout;

pub fn editor_id() -> Id {
    Id::new("calendar-editor")
}

/// What the calendar draws from, loaded by the app for the current range.
#[derive(Debug, Clone)]
pub struct CalendarData {
    pub entries: Vec<EntryRow>,
    pub days: Vec<(Date, Totals)>,
    pub schedule: WeeklySchedule,
    /// Time after this is the in-progress interval: not a gap.
    pub logged_until: Timestamp,
}

#[derive(Debug, Clone)]
pub enum Message {
    Unit(RangeUnit),
    Step(i32),
    OpenDay(Date),
    SelectBlock(Block),
    SelectGap { start: Timestamp, end: Timestamp },
    Input(String),
    Save,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    RangeChanged,
    Rename {
        id: ActivityId,
        to: ActivityName,
    },
    Merge {
        from: ActivityId,
        into: ActivityId,
    },
    LogTime {
        activity: ActivityName,
        start: Timestamp,
        end: Timestamp,
    },
    Focus(Id),
}

#[derive(Debug, Clone)]
enum Target {
    Block(Block),
    Gap { start: Timestamp, end: Timestamp },
}

#[derive(Debug, Clone)]
struct Editor {
    target: Target,
    input: String,
    invalid: bool,
    /// The new name is taken by this activity; Save again merges into it.
    merge_into: Option<ActivityId>,
}

#[derive(Debug, Clone)]
pub struct Calendar {
    spec: RangeSpec,
    today: Date,
    tz: TimeZone,
    data: Option<CalendarData>,
    editor: Option<Editor>,
    error: Option<String>,
}

impl Calendar {
    pub fn new(today: Date, tz: TimeZone) -> Self {
        Self {
            spec: RangeSpec::new(RangeUnit::Week, today),
            today,
            tz,
            data: None,
            editor: None,
            error: None,
        }
    }

    pub fn spec(&self) -> RangeSpec {
        self.spec
    }

    /// New data from the store. `today` moves at midnight.
    pub fn set_data(&mut self, data: CalendarData, today: Date) {
        self.data = Some(data);
        self.today = today;
    }

    /// The store said a rename collides with `existing`: Save again merges into it.
    pub fn conflict(&mut self, existing: ActivityId) {
        if let Some(editor) = &mut self.editor {
            editor.merge_into = Some(existing);
        }
    }

    /// Success closes the editor; failure keeps it open with the reason.
    pub fn saved(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.editor = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    #[cfg(test)]
    fn editor_input(&self) -> Option<&str> {
        self.editor.as_ref().map(|e| e.input.as_str())
    }

    fn set_spec(&mut self, spec: RangeSpec) -> Option<Outcome> {
        self.spec = spec;
        self.editor = None;
        self.error = None;
        Some(Outcome::RangeChanged)
    }

    fn open(&mut self, target: Target, input: String) -> Option<Outcome> {
        self.error = None;
        self.editor = Some(Editor {
            target,
            input,
            invalid: false,
            merge_into: None,
        });
        Some(Outcome::Focus(editor_id()))
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Unit(unit @ (RangeUnit::Day | RangeUnit::Week | RangeUnit::Month)) => {
                self.set_spec(RangeSpec::new(unit, self.spec.anchor))
            }
            Message::Unit(_) => None,
            Message::Step(steps) => {
                let next = step_within(self.spec, steps, self.today)?;
                self.set_spec(next)
            }
            Message::OpenDay(day) => self.set_spec(RangeSpec::new(RangeUnit::Day, day)),
            Message::SelectBlock(block) => {
                let input = block.name.to_string();
                self.open(Target::Block(block), input)
            }
            Message::SelectGap { start, end } => {
                self.open(Target::Gap { start, end }, String::new())
            }
            Message::Input(value) => {
                if let Some(editor) = &mut self.editor {
                    editor.input = value;
                    editor.invalid = false;
                    editor.merge_into = None;
                }
                None
            }
            Message::Save => {
                let editor = self.editor.as_mut()?;
                let name = ActivityName::parse(&editor.input);
                let outcome = match (&editor.target, name) {
                    (Target::Block(block), name) => match (editor.merge_into, name) {
                        (Some(into), _) => Some(Outcome::Merge {
                            from: block.activity,
                            into,
                        }),
                        (None, Ok(to)) => Some(Outcome::Rename {
                            id: block.activity,
                            to,
                        }),
                        (None, Err(_)) => None,
                    },
                    (Target::Gap { start, end }, Ok(activity)) => Some(Outcome::LogTime {
                        activity,
                        start: *start,
                        end: *end,
                    }),
                    (Target::Gap { .. }, Err(_)) => None,
                };
                editor.invalid = outcome.is_none();
                outcome
            }
            Message::Cancel => {
                self.editor = None;
                self.error = None;
                None
            }
        }
    }

    /// Esc closes the editor; ←/→ step the range while no editor is open.
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        let typing = self.editor.is_some();
        match press.key.as_ref() {
            Key::Named(Named::Escape) => self.update(Message::Cancel),
            Key::Named(Named::ArrowLeft) if !typing => self.update(Message::Step(-1)),
            Key::Named(Named::ArrowRight) if !typing => self.update(Message::Step(1)),
            _ => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut page = column![self.header()].spacing(theme::S_MD);
        if let Some(error) = &self.error {
            page = page.push(theme::label(error).color(theme::ALERT));
        }
        if let Some(data) = &self.data {
            page = page.push(match self.spec.unit {
                RangeUnit::Day => self.day_view(data),
                RangeUnit::Month => self.month_view(data),
                _ => self.week_view(data),
            });
        }
        page.height(Length::Fill).into()
    }

    #[cfg(test)]
    fn day_slots(&self) -> Vec<Slot> {
        self.data
            .as_ref()
            .map(|data| self.day_columns(data).1)
            .unwrap_or_default()
    }

    /// The first and last day the range covers.
    fn bounds(&self) -> (Date, Date) {
        self.spec
            .days()
            .unwrap_or((self.spec.anchor, self.spec.anchor))
    }

    /// The days the range covers.
    fn view_days(&self) -> Vec<Date> {
        let (first, last) = self.bounds();
        first.series(1.day()).take_while(|d| *d <= last).collect()
    }

    /// Each of `days` cut into slots over the hours they all show.
    fn columns(&self, data: &CalendarData, days: &[Date]) -> Columns {
        let blocks: Vec<_> = days
            .iter()
            .map(|day| (*day, layout::blocks(&data.entries, *day, &self.tz)))
            .collect();
        let hours = layout::visible_hours(&blocks, &data.schedule, &self.tz);
        let columns = blocks
            .iter()
            .map(|(day, blocks)| {
                let window = layout::window(*day, hours, &self.tz);
                let working = data.schedule.period_on(*day, &self.tz);
                (
                    *day,
                    layout::slots(blocks, window, working, data.logged_until),
                )
            })
            .collect();
        (hours, columns)
    }

    /// The Day timeline's hours and slots (for a non-Day range: those of the anchor).
    fn day_columns(&self, data: &CalendarData) -> ((i8, i8), Vec<Slot>) {
        let (hours, mut columns) = self.columns(data, &[self.spec.anchor]);
        (hours, columns.pop().map(|c| c.1).unwrap_or_default())
    }

    fn header(&self) -> Element<'_, Message> {
        let unit = |unit, name: &'static str| {
            let selected = self.spec.unit == unit;
            let font = if selected {
                theme::UI_SEMIBOLD
            } else {
                theme::UI
            };
            button(
                container(text(name).size(13).font(font))
                    .height(Length::Fill)
                    .align_y(Vertical::Center),
            )
            .height(26)
            .padding(Padding::from([0.0, 10.0]))
            .style(theme::segment(selected))
            .on_press(Message::Unit(unit))
        };
        let step = |handle, steps| step_arrow(handle, self.spec, steps, self.today, Message::Step);
        let total = self.data.as_ref().map_or(0, |data| {
            let (first, last) = self.bounds();
            data.days
                .iter()
                .filter(|(day, _)| (first..=last).contains(day))
                .map(|(_, totals)| totals.seconds)
                .sum()
        });
        row![
            theme::title("Calendar"),
            container(
                row![
                    unit(RangeUnit::Day, "Day"),
                    unit(RangeUnit::Week, "Week"),
                    unit(RangeUnit::Month, "Month"),
                ]
                .spacing(theme::S_HAIR)
            )
            .padding(3)
            .style(theme::raised_panel),
            Space::new().width(Length::Fill),
            step(icons::chevron_left(), -1),
            text(format::range_title(self.spec, self.today).1)
                .font(theme::UI_SEMIBOLD)
                .size(14)
                .width(112)
                .align_x(Horizontal::Center),
            step(icons::chevron_right(), 1),
            text(format::duration(total))
                .font(theme::MONO_BOLD)
                .size(13),
        ]
        .spacing(theme::S_MD)
        .align_y(Vertical::Center)
        .into()
    }

    fn is_working(&self, data: &CalendarData, day: Date) -> bool {
        data.schedule.period_on(day, &self.tz).is_some()
    }

    fn week_view<'a>(&'a self, data: &'a CalendarData) -> Element<'a, Message> {
        let days = self.view_days();
        let (hours, columns) = self.columns(data, &days);
        let heads = days.iter().map(|day| {
            let color = if *day == self.today {
                theme::LAMP_AMBER
            } else {
                theme::TEXT_MUTED
            };
            let seconds = totals(data, *day).seconds;
            column![
                text(day.strftime("%a %-d").to_string())
                    .size(12)
                    .color(color),
                text(if seconds > 0 {
                    format::duration(seconds)
                } else {
                    String::new()
                })
                .font(theme::MONO)
                .size(11)
                .color(theme::TEXT_MUTED),
            ]
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .into()
        });
        let heads = row![Space::new().width(40)]
            .extend(heads)
            .spacing(theme::S_XS);
        let body = row![hour_labels(hours)]
            .extend(columns.into_iter().map(|(day, slots)| {
                let content = column(slots.into_iter().map(|slot| {
                    match slot {
                        Slot::Block(block) => {
                            let p = portion(block.start, block.end);
                            let label: Element<'a, Message> = if p >= 30 {
                                column![
                                    text(block.name.name.clone())
                                        .font(theme::UI_SEMIBOLD)
                                        .size(11),
                                    text(format::duration(secs(block.start, block.end)))
                                        .font(theme::MONO)
                                        .size(11)
                                        .color(theme::TEXT_MUTED),
                                ]
                                .into()
                            } else {
                                Space::new().into()
                            };
                            button(label)
                                .width(Length::Fill)
                                .height(Length::FillPortion(p))
                                .padding(Padding::from([2.0, 4.0]))
                                .clip(true)
                                .style(theme::calendar_block(category_color(
                                    block.name.category.as_deref(),
                                )))
                                .on_press(Message::OpenDay(day))
                                .into()
                        }
                        Slot::Gap { start, end } | Slot::Empty { start, end } => Space::new()
                            .height(Length::FillPortion(portion(start, end)))
                            .into(),
                    }
                }));
                self.timeline(data, day, hours, content.into())
            }))
            .spacing(theme::S_XS)
            .height(Length::Fill);
        column![heads, body, self.legend(data)]
            .spacing(theme::S_SM)
            .into()
    }

    /// A day's hour lines with `content` on top, in the day's surface.
    fn timeline<'a>(
        &self,
        data: &CalendarData,
        day: Date,
        hours: (i8, i8),
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        container(
            stack![hour_lines(hours), content]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::calendar_day(
            self.is_working(data, day),
            day == self.today,
        ))
        .into()
    }

    fn day_view<'a>(&'a self, data: &'a CalendarData) -> Element<'a, Message> {
        let day = self.spec.anchor;
        let (hours, slots) = self.day_columns(data);
        let content = column(slots.iter().map(|slot| {
            match slot {
                Slot::Block(block) => {
                    let p = portion(block.start, block.end);
                    let mut label = row![
                        text(block.name.name.clone())
                            .font(theme::UI_SEMIBOLD)
                            .size(13)
                    ]
                    .spacing(theme::S_SM)
                    .align_y(Vertical::Center);
                    if let Some(category) = &block.name.category {
                        label =
                            label.push(text(category.clone()).size(11).color(theme::TEXT_MUTED));
                    }
                    label = label.push(Space::new().width(Length::Fill)).push(
                        text(format!(
                            "{}–{} · {}",
                            format::clock(block.start, &self.tz),
                            format::clock(block.end, &self.tz),
                            format::duration(secs(block.start, block.end)),
                        ))
                        .font(theme::MONO)
                        .size(11)
                        .color(theme::TEXT_MUTED),
                    );
                    button(label)
                        .width(Length::Fill)
                        .height(Length::FillPortion(p))
                        .padding(Padding::from([2.0, 8.0]))
                        .clip(true)
                        .style(theme::calendar_block(category_color(
                            block.name.category.as_deref(),
                        )))
                        .on_press(Message::SelectBlock(block.clone()))
                        .into()
                }
                Slot::Gap { start, end } => button(
                    row![
                        text("Not logged · ").size(11),
                        text(format::duration(secs(*start, *end)))
                            .font(theme::MONO)
                            .size(11),
                    ]
                    .align_y(Vertical::Center),
                )
                .width(Length::Fill)
                .height(Length::FillPortion(portion(*start, *end)))
                .padding(Padding::from([2.0, 8.0]))
                .clip(true)
                .style(theme::calendar_gap)
                .on_press(Message::SelectGap {
                    start: *start,
                    end: *end,
                })
                .into(),
                Slot::Empty { start, end } => Space::new()
                    .height(Length::FillPortion(portion(*start, *end)))
                    .into(),
            }
        }));
        row![
            hour_labels(hours),
            self.timeline(data, day, hours, content.into()),
            self.day_panel(data, day, &slots),
        ]
        .spacing(theme::S_XS)
        .height(Length::Fill)
        .into()
    }

    fn day_panel<'a>(
        &'a self,
        data: &'a CalendarData,
        day: Date,
        slots: &[Slot],
    ) -> Element<'a, Message> {
        let panel = match &self.editor {
            Some(editor) => self.editor_panel(editor),
            None => {
                let totals = totals(data, day);
                let sum = |want_gaps: bool| -> i64 {
                    slots
                        .iter()
                        .map(|slot| match slot {
                            Slot::Gap { start, end } if want_gaps => secs(*start, *end),
                            Slot::Block(b) if !want_gaps => secs(b.start, b.end),
                            _ => 0,
                        })
                        .sum()
                };
                let (gaps, blocks) = (sum(true), sum(false));
                let mut panel = column![
                    theme::label("Tracked"),
                    theme::figure_display(format::duration(totals.seconds)),
                ]
                .spacing(theme::S_XS);
                if gaps > 0 {
                    panel = panel.push(
                        theme::small(format!("{} not logged", format::duration(gaps)))
                            .color(theme::LAMP_AMBER_INK_MUTED),
                    );
                }
                if totals.seconds > blocks {
                    panel = panel.push(theme::label(format!(
                        "{} added without a time",
                        format::duration(totals.seconds - blocks)
                    )));
                }
                panel = panel.push(divider());
                for line in dashboard::lines(totals) {
                    let name = theme::small(line.name);
                    let name = if line.kind == LineKind::Category {
                        name.font(theme::UI_SEMIBOLD)
                    } else {
                        name
                    };
                    let indent = if line.kind == LineKind::Activity {
                        theme::S_MD
                    } else {
                        0.0
                    };
                    panel = panel.push(
                        row![
                            Space::new().width(indent),
                            name.width(Length::Fill),
                            text(format::duration(line.seconds))
                                .font(theme::MONO)
                                .size(12),
                        ]
                        .spacing(theme::S_SM),
                    );
                }
                panel.push(theme::label(
                    "Click a block to rename it or move it to another category. \
                     Click a gap to log what that time was.",
                ))
            }
        };
        scrollable(panel.spacing(theme::S_SM).width(200))
            .style(theme::scroll)
            .into()
    }

    fn editor_panel<'a>(&'a self, editor: &'a Editor) -> iced::widget::Column<'a, Message> {
        let heading = match &editor.target {
            Target::Block(_) => String::from("Rename"),
            Target::Gap { start, end } => format!(
                "Log {}–{}",
                format::clock(*start, &self.tz),
                format::clock(*end, &self.tz)
            ),
        };
        let field = text_input("Category: Activity", &editor.input)
            .id(editor_id())
            .on_input(Message::Input)
            .on_submit(Message::Save)
            .font(theme::UI)
            .size(14)
            .padding(Padding::from([9.0, 10.0]))
            .style(theme::inset_field(widgets::look(editor.invalid, false)));
        let merging = editor.merge_into.is_some();
        // A gap being logged has nothing to say until something goes wrong.
        let quiet = matches!(editor.target, Target::Gap { .. }) && !merging && !editor.invalid;
        let hint = (!quiet).then(|| widgets::rename_hint(merging, editor.invalid));
        column![theme::title(heading), field]
            .extend(hint.map(Element::from))
            .push(
                row![
                    ghost_button("Cancel", None, Some(Message::Cancel)),
                    primary_button(String::from("Save"), None, None, Some(Message::Save)),
                ]
                .spacing(theme::S_SM),
            )
    }

    fn month_view<'a>(&'a self, data: &'a CalendarData) -> Element<'a, Message> {
        let (first, last) = self.bounds();
        let weekdays = row(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
            .map(|name| theme::label(name).width(Length::Fill).into()))
        .padding(Padding::from([0.0, 8.0]))
        .spacing(theme::S_XS);
        let grid = column(layout::month_grid(self.spec.anchor).chunks(7).map(|week| {
            row(week
                .iter()
                .map(|day| self.month_cell(data, *day, (first..=last).contains(day))))
            .spacing(theme::S_XS)
            .height(Length::Fill)
            .into()
        }))
        .spacing(theme::S_XS)
        .height(Length::Fill);
        column![weekdays, grid, self.legend(data)]
            .spacing(theme::S_SM)
            .into()
    }

    fn month_cell<'a>(
        &self,
        data: &'a CalendarData,
        day: Date,
        in_month: bool,
    ) -> Element<'a, Message> {
        let today = day == self.today;
        let summary = layout::summary(totals(data, day));
        let date = text(day.day().to_string()).size(13).font(if today {
            theme::UI_SEMIBOLD
        } else {
            theme::UI
        });
        let date = if today {
            date.color(theme::LAMP_AMBER)
        } else {
            date
        };
        let total: Element<'a, Message> = if summary.seconds > 0 {
            text(format::duration(summary.seconds))
                .font(theme::MONO)
                .size(11)
                .into()
        } else {
            Space::new().into()
        };
        let bar = row(summary.parts.iter().map(|(category, seconds)| {
            container(Space::new())
                .width(Length::FillPortion(
                    u16::try_from((seconds / 60).max(1)).unwrap_or(u16::MAX),
                ))
                .height(4)
                .style(theme::bar_fill(category_color(category.as_deref())))
                .into()
        }))
        .spacing(1)
        .height(4);
        let mut content = column![
            row![date, Space::new().width(Length::Fill), total].align_y(Vertical::Center),
            bar,
        ]
        .spacing(theme::S_XS);
        if let Some(top) = summary.top {
            content = content.push(text(top).size(11).color(theme::TEXT_MUTED));
        }
        button(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .clip(true)
            .style(theme::month_cell(
                in_month,
                self.is_working(data, day),
                today,
            ))
            .on_press_maybe((in_month && day <= self.today).then_some(Message::OpenDay(day)))
            .into()
    }

    /// A swatch and name per category in view, "No category" last.
    fn legend<'a>(&self, data: &CalendarData) -> Element<'a, Message> {
        let mut categories: Vec<Option<String>> = Vec::new();
        for day in self.view_days() {
            for group in &totals(data, day).groups {
                if !categories.contains(&group.category) {
                    categories.push(group.category.clone());
                }
            }
        }
        categories.sort_by_key(Option::is_none);
        row(categories.into_iter().map(|category| {
            row![
                container(Space::new())
                    .width(8)
                    .height(8)
                    .style(theme::bar_fill(category_color(category.as_deref()))),
                theme::label(category.unwrap_or_else(|| String::from("No category"))),
            ]
            .spacing(theme::S_XS)
            .align_y(Vertical::Center)
            .into()
        }))
        .spacing(theme::S_MD)
        .into()
    }
}

/// The shared hours, and each day's slots.
type Columns = ((i8, i8), Vec<(Date, Vec<Slot>)>);

static EMPTY: Totals = Totals {
    groups: Vec::new(),
    seconds: 0,
    first: None,
    last: None,
};

/// The totals recorded for `day`; a day with none is empty.
fn totals(data: &CalendarData, day: Date) -> &Totals {
    data.days
        .iter()
        .find(|(date, _)| *date == day)
        .map_or(&EMPTY, |(_, totals)| totals)
}

fn secs(start: Timestamp, end: Timestamp) -> i64 {
    end.duration_since(start).as_secs()
}

/// One hairline per visible hour, each the same share of the height.
fn hour_lines<'a>(hours: (i8, i8)) -> Element<'a, Message> {
    column((hours.0..hours.1).map(|_| {
        column![divider(), Space::new().height(Length::Fill)]
            .height(Length::FillPortion(60))
            .into()
    }))
    .into()
}

/// The 40px gutter of hour labels, each at the top of its hour.
fn hour_labels<'a>(hours: (i8, i8)) -> Element<'a, Message> {
    column((hours.0..hours.1).map(|hour| {
        container(
            text(format!("{hour:02}"))
                .font(theme::MONO)
                .size(11)
                .color(theme::TEXT_PLACEHOLDER),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(60))
        .align_x(Horizontal::Right)
        .padding(Padding::from([0.0, 6.0]))
        .into()
    }))
    .width(40)
    .into()
}

#[cfg(test)]
mod tests;
