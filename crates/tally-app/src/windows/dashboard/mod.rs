//! The tray popover: totals for a range, edited in place. Artboards B-Dashboard,
//! B-Dashboard-Edit, B-Range and B-AddTime.

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{
    Id, Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text,
    text_input,
};
use iced::{Element, Length, Padding, Size};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use tally_core::ActivityName;
use tally_core::duration::{DurationEdit, parse_amount, parse_edit};
use tally_core::range::{RangeSpec, RangeUnit};
use tally_core::store::{ActivityId, Totals};

use super::KeyPress;
use super::widgets::{
    self, bar, footer, ghost_button, icon_button, primary_button, status_line, step_arrow,
};
use crate::status::Status;
use crate::{format, icons, theme};

pub const SIZE: Size = Size::new(400.0, 620.0);
/// Width of the bar column and the figure column (B-Dashboard grid: `1fr 88px 64px`).
const BAR_WIDTH: f32 = 88.0;
const FIGURE_WIDTH: f32 = 64.0;

pub fn edit_id() -> Id {
    Id::new("dashboard-edit")
}

pub fn rename_id() -> Id {
    Id::new("dashboard-rename")
}

pub fn add_activity_id() -> Id {
    Id::new("dashboard-add-activity")
}

pub fn add_duration_id() -> Id {
    Id::new("dashboard-add-duration")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
}

pub const PRESETS: [(Preset, &str); 4] = [
    (Preset::Today, "Today"),
    (Preset::Yesterday, "Yesterday"),
    (Preset::ThisWeek, "This week"),
    (Preset::LastWeek, "Last week"),
];

pub fn preset_spec(preset: Preset, today: Date) -> RangeSpec {
    let step = |unit, steps| {
        RangeSpec::new(unit, today)
            .step(steps)
            .unwrap_or(RangeSpec::new(unit, today))
    };
    match preset {
        Preset::Today => RangeSpec::new(RangeUnit::Day, today),
        Preset::Yesterday => step(RangeUnit::Day, -1),
        Preset::ThisWeek => RangeSpec::new(RangeUnit::Week, today),
        Preset::LastWeek => step(RangeUnit::Week, -1),
    }
}

/// The preset that shows the same days as `spec`, for the menu's check mark.
pub fn current_preset(spec: RangeSpec, today: Date) -> Option<Preset> {
    PRESETS.iter().map(|(p, _)| *p).find(|preset| {
        let candidate = preset_spec(*preset, today);
        candidate.unit == spec.unit && candidate.days().ok() == spec.days().ok()
    })
}

/// The range one ‹ or › away, if it can move. "All" never moves, and the future is
/// off limits.
pub fn step_within(spec: RangeSpec, steps: i32, today: Date) -> Option<RangeSpec> {
    if spec.unit == RangeUnit::All {
        return None;
    }
    let next = spec.step(steps).ok()?;
    let (first, _) = next.days().ok()?;
    (steps < 0 || first <= today).then_some(next)
}

/// What a total becomes after an edit.
pub fn preview(current: i64, edit: DurationEdit) -> i64 {
    match edit {
        DurationEdit::Set(target) => target,
        DurationEdit::Add(amount) => current + amount,
        DurationEdit::Subtract(amount) => (current - amount).max(0),
    }
}

/// The add-time duration: an unsigned amount above zero (`1h 30`, `90`, `45m`).
pub fn parse_add_duration(input: &str) -> Option<i64> {
    parse_amount(input.trim().trim_start_matches('+')).filter(|seconds| *seconds > 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Category,
    /// An activity inside a category, indented.
    Activity,
    /// An uncategorised activity at the top level.
    Single,
}

/// One row of the totals list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    /// Set for editable rows.
    pub activity: Option<ActivityId>,
    pub name: String,
    pub seconds: i64,
    /// The bar is `seconds` out of `bar_max`: categories and top-level activities
    /// against the range total, activities against their category.
    pub bar_max: i64,
}

pub fn lines(totals: &Totals) -> Vec<Line> {
    let mut lines = Vec::new();
    for group in &totals.groups {
        match &group.category {
            Some(category) => {
                lines.push(Line {
                    kind: LineKind::Category,
                    activity: None,
                    name: category.clone(),
                    seconds: group.seconds,
                    bar_max: totals.seconds,
                });
                lines.extend(group.activities.iter().map(|a| Line {
                    kind: LineKind::Activity,
                    activity: Some(a.id),
                    name: a.name.name.clone(),
                    seconds: a.seconds,
                    bar_max: group.seconds,
                }));
            }
            None => lines.extend(group.activities.iter().map(|a| Line {
                kind: LineKind::Single,
                activity: Some(a.id),
                name: a.name.name.clone(),
                seconds: a.seconds,
                bar_max: totals.seconds,
            })),
        }
    }
    lines
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleRangeMenu,
    Preset(Preset),
    Unit(RangeUnit),
    Step(i32),
    CloseMenus,
    EditTotal(ActivityId),
    EditInput(String),
    SaveEdit,
    Rename(ActivityId),
    RenameCategory(String),
    RenameInput(String),
    SaveRename,
    StartAdd,
    AddActivity(String),
    AddDuration(String),
    SaveAdd,
    Cancel,
    TogglePause,
    EndDay,
    ToggleMenu,
    OpenMain,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Reload totals for the dashboard's new range.
    RangeChanged,
    EditTotal {
        activity: ActivityId,
        edit: DurationEdit,
    },
    AddTime {
        activity: ActivityName,
        seconds: i64,
    },
    Rename {
        id: ActivityId,
        to: ActivityName,
    },
    Merge {
        from: ActivityId,
        into: ActivityId,
    },
    RenameCategory {
        from: String,
        to: String,
    },
    Focus(Id),
    TogglePause,
    EndDay,
    OpenMain,
    Quit,
    Close,
}

#[derive(Debug, Clone)]
struct Edit {
    activity: ActivityId,
    name: String,
    current: i64,
    input: String,
    invalid: bool,
}

#[derive(Debug, Clone)]
struct Renaming {
    target: Target,
    input: String,
    invalid: bool,
    /// The new name is taken by this activity; Enter again merges into it.
    merge_into: Option<ActivityId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Activity(ActivityId),
    Category(String),
}

#[derive(Debug, Clone, Default)]
struct Add {
    activity: String,
    duration: String,
    invalid: bool,
}

/// The one inline editor that can be open.
#[derive(Debug, Clone)]
enum Editor {
    Total(Edit),
    Rename(Renaming),
    Add(Add),
}

#[derive(Debug, Clone)]
pub struct Dashboard {
    spec: RangeSpec,
    today: Date,
    tz: TimeZone,
    totals: Totals,
    status: Status,
    paused: bool,
    range_menu: bool,
    menu: bool,
    /// Keyboard selection among editable rows.
    selected: Option<usize>,
    editor: Option<Editor>,
    error: Option<String>,
}

impl Dashboard {
    pub fn new(today: Date, tz: TimeZone) -> Self {
        Self {
            spec: RangeSpec::new(RangeUnit::Day, today),
            today,
            tz,
            totals: Totals::default(),
            status: Status::default(),
            paused: false,
            range_menu: false,
            menu: false,
            selected: None,
            editor: None,
            error: None,
        }
    }

    pub fn spec(&self) -> RangeSpec {
        self.spec
    }

    /// The day an added correction is filed under for the current range.
    pub fn correction_day(&self) -> Date {
        self.spec.correction_day(self.today).unwrap_or(self.today)
    }

    /// New data from the store and scheduler. `today` moves at midnight.
    pub fn set_data(&mut self, totals: Totals, status: Status, paused: bool, today: Date) {
        self.totals = totals;
        self.status = status;
        self.paused = paused;
        self.today = today;
    }

    /// The app calls this after a save: success closes the editor, failure keeps it
    /// open with the reason, so nothing typed is lost.
    pub fn saved(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.editor = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// The store said a rename collides with `existing`: Enter again merges into it.
    pub fn conflict(&mut self, existing: ActivityId) {
        if let Some(Editor::Rename(rename)) = &mut self.editor {
            rename.merge_into = Some(existing);
        }
    }

    fn start_rename(&mut self, target: Target, input: String) -> Option<Outcome> {
        self.error = None;
        self.editor = Some(Editor::Rename(Renaming {
            target,
            input,
            invalid: false,
            merge_into: None,
        }));
        Some(Outcome::Focus(rename_id()))
    }

    fn activity_name(&self, id: ActivityId) -> Option<&ActivityName> {
        self.totals
            .groups
            .iter()
            .flat_map(|g| &g.activities)
            .find(|a| a.id == id)
            .map(|a| &a.name)
    }

    fn editable(&self) -> Vec<(ActivityId, String, i64)> {
        lines(&self.totals)
            .into_iter()
            .filter_map(|l| l.activity.map(|id| (id, l.name, l.seconds)))
            .collect()
    }

    fn set_spec(&mut self, spec: RangeSpec) -> Option<Outcome> {
        self.spec = spec;
        self.range_menu = false;
        // An add in progress survives a range change; row editors belong to the old rows.
        if !matches!(self.editor, Some(Editor::Add(_))) {
            self.editor = None;
        }
        self.selected = None;
        Some(Outcome::RangeChanged)
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::ToggleRangeMenu => {
                self.range_menu = !self.range_menu;
                self.menu = false;
                None
            }
            Message::Preset(preset) => self.set_spec(preset_spec(preset, self.today)),
            // Keep the anchor, so switching to Week while viewing a day shows that day's week.
            Message::Unit(unit) => self.set_spec(RangeSpec::new(unit, self.spec.anchor)),
            Message::Step(steps) => {
                let next = step_within(self.spec, steps, self.today)?;
                self.set_spec(next)
            }
            Message::CloseMenus => {
                self.range_menu = false;
                self.menu = false;
                None
            }
            Message::EditTotal(activity) => {
                let (_, name, current) = self
                    .editable()
                    .into_iter()
                    .find(|(id, _, _)| *id == activity)?;
                self.error = None;
                self.editor = Some(Editor::Total(Edit {
                    activity,
                    name,
                    current,
                    input: String::new(),
                    invalid: false,
                }));
                Some(Outcome::Focus(edit_id()))
            }
            Message::EditInput(value) => {
                if let Some(Editor::Total(edit)) = &mut self.editor {
                    edit.input = value;
                    edit.invalid = false;
                }
                None
            }
            Message::SaveEdit => {
                let Some(Editor::Total(edit)) = &mut self.editor else {
                    return None;
                };
                match parse_edit(&edit.input) {
                    Ok(parsed) => Some(Outcome::EditTotal {
                        activity: edit.activity,
                        edit: parsed,
                    }),
                    Err(_) => {
                        edit.invalid = true;
                        None
                    }
                }
            }
            Message::Rename(activity) => {
                let input = self.activity_name(activity)?.to_string();
                self.start_rename(Target::Activity(activity), input)
            }
            Message::RenameCategory(category) => {
                self.start_rename(Target::Category(category.clone()), category)
            }
            Message::RenameInput(value) => {
                if let Some(Editor::Rename(rename)) = &mut self.editor {
                    rename.input = value;
                    rename.invalid = false;
                    rename.merge_into = None;
                }
                None
            }
            Message::SaveRename => {
                let Some(Editor::Rename(rename)) = &mut self.editor else {
                    return None;
                };
                let outcome = match &rename.target {
                    Target::Activity(id) => {
                        match (rename.merge_into, ActivityName::parse(&rename.input)) {
                            (Some(into), _) => Some(Outcome::Merge { from: *id, into }),
                            (None, Ok(to)) => Some(Outcome::Rename { id: *id, to }),
                            (None, Err(_)) => None,
                        }
                    }
                    Target::Category(from) => {
                        let to = rename.input.trim();
                        (!to.is_empty() && !to.contains(": ")).then(|| Outcome::RenameCategory {
                            from: from.clone(),
                            to: to.to_owned(),
                        })
                    }
                };
                rename.invalid = outcome.is_none();
                outcome
            }
            Message::StartAdd => {
                self.menu = false;
                self.error = None;
                self.editor = Some(Editor::Add(Add::default()));
                Some(Outcome::Focus(add_activity_id()))
            }
            Message::AddActivity(value) => {
                if let Some(Editor::Add(add)) = &mut self.editor {
                    add.activity = value;
                    add.invalid = false;
                }
                None
            }
            Message::AddDuration(value) => {
                if let Some(Editor::Add(add)) = &mut self.editor {
                    add.duration = value;
                    add.invalid = false;
                }
                None
            }
            Message::SaveAdd => {
                let Some(Editor::Add(add)) = &mut self.editor else {
                    return None;
                };
                match (
                    ActivityName::parse(&add.activity),
                    parse_add_duration(&add.duration),
                ) {
                    (Ok(activity), Some(seconds)) => Some(Outcome::AddTime { activity, seconds }),
                    _ => {
                        add.invalid = true;
                        None
                    }
                }
            }
            Message::Cancel => {
                if self.range_menu || self.menu {
                    self.range_menu = false;
                    self.menu = false;
                    None
                } else if self.editor.is_some() {
                    self.editor = None;
                    self.error = None;
                    None
                } else {
                    Some(Outcome::Close)
                }
            }
            Message::TogglePause => Some(Outcome::TogglePause),
            Message::EndDay => {
                self.menu = false;
                Some(Outcome::EndDay)
            }
            Message::ToggleMenu => {
                self.menu = !self.menu;
                self.range_menu = false;
                None
            }
            Message::OpenMain => {
                self.menu = false;
                Some(Outcome::OpenMain)
            }
            Message::Quit => Some(Outcome::Quit),
        }
    }

    /// Esc backs out one level; ←/→ step the range; ↑/↓ choose a row and Enter edits
    /// it; Tab moves between the add-time fields.
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        let typing = self.editor.is_some();
        match press.key.as_ref() {
            Key::Named(Named::Escape) => self.update(Message::Cancel),
            Key::Named(Named::ArrowLeft) if !typing => self.update(Message::Step(-1)),
            Key::Named(Named::ArrowRight) if !typing => self.update(Message::Step(1)),
            Key::Named(Named::ArrowUp) if !typing => {
                self.selected = Some(self.selected.map_or(0, |s| s.saturating_sub(1)));
                None
            }
            Key::Named(Named::ArrowDown) if !typing => {
                let last = self.editable().len().checked_sub(1)?;
                self.selected = Some(self.selected.map_or(0, |s| (s + 1).min(last)));
                None
            }
            Key::Named(Named::Enter) if !typing && !press.captured => {
                let (id, _, _) = self.editable().get(self.selected?)?.clone();
                self.update(Message::EditTotal(id))
            }
            Key::Named(Named::Tab) if matches!(self.editor, Some(Editor::Add(_))) => {
                Some(Outcome::Focus(if press.modifiers.shift() {
                    add_activity_id()
                } else {
                    add_duration_id()
                }))
            }
            _ => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = column![
            self.header(),
            container(widgets::unit_picker(self.spec.unit, true, Message::Unit)).padding(Padding {
                top: 8.0,
                right: 12.0,
                bottom: 0.0,
                left: 12.0
            }),
            self.total(),
            widgets::divider(),
            scrollable(container(self.list()).padding(Padding::from([10.0, 8.0])))
                .height(Length::Fill)
                .style(theme::scroll),
            footer(self.actions()),
        ];
        let mut layers = stack![base].width(Length::Fill).height(Length::Fill);
        if self.range_menu {
            layers = layers.push(self.scrim(56.0));
            layers = layers.push(column![
                Space::new().height(56),
                row![Space::new().width(8), opaque(self.range_menu_view())],
            ]);
        }
        if self.menu {
            layers = layers.push(self.scrim(0.0));
            layers = layers.push(column![
                Space::new().height(Length::Fill),
                row![
                    Space::new().width(Length::Fill),
                    opaque(self.main_menu_view()),
                    Space::new().width(8)
                ],
                Space::new().height(60),
            ]);
        }
        container(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::popup_frame)
            .into()
    }

    /// A click-catching dim layer below `top` px; clicking it closes the menus.
    fn scrim(&self, top: f32) -> Element<'_, Message> {
        column![
            mouse_area(Space::new().width(Length::Fill).height(top)).on_press(Message::CloseMenus),
            mouse_area(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::scrim)
            )
            .on_press(Message::CloseMenus),
        ]
        .into()
    }

    fn header(&self) -> Element<'_, Message> {
        let (title, subtitle) = format::range_title(self.spec, self.today);
        let picker = button(
            row![
                column![theme::title(title), theme::label(subtitle)],
                widgets::icon(icons::chevron_down(), 14.0, theme::TEXT_MUTED),
            ]
            .spacing(theme::S_SM)
            .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .height(44)
        .padding(Padding::from([0.0, 8.0]))
        .style(if self.range_menu {
            theme::button_ghost_on
        } else {
            theme::button_ghost
        })
        .on_press(Message::ToggleRangeMenu);
        let step = |handle, steps| step_arrow(handle, self.spec, steps, self.today, Message::Step);
        container(
            row![
                picker,
                step(icons::chevron_left(), -1),
                step(icons::chevron_right(), 1)
            ]
            .spacing(theme::S_HAIR)
            .align_y(Vertical::Center),
        )
        .padding(Padding {
            top: 8.0,
            right: 8.0,
            bottom: 0.0,
            left: 8.0,
        })
        .into()
    }

    fn total(&self) -> Element<'_, Message> {
        let mut line = row![theme::figure_display(format::duration(self.totals.seconds))]
            .spacing(10)
            .align_y(Vertical::Bottom);
        if let (Some(first), Some(last)) = (self.totals.first, self.totals.last) {
            line = line.push(
                row![
                    theme::small("tracked, ").color(theme::TEXT_MUTED),
                    text(format::span(first, last, &self.tz))
                        .font(theme::MONO)
                        .size(13)
                        .color(theme::TEXT_MUTED),
                ]
                .align_y(Vertical::Center),
            );
        } else {
            line = line.push(theme::small("tracked").color(theme::TEXT_MUTED));
        }
        container(line)
            .padding(Padding {
                top: 20.0,
                right: 16.0,
                bottom: 16.0,
                left: 16.0,
            })
            .into()
    }

    fn list(&self) -> Element<'_, Message> {
        let all = lines(&self.totals);
        if all.is_empty() && !matches!(self.editor, Some(Editor::Add(_))) {
            return container(
                theme::small("Nothing tracked in this range yet.").color(theme::TEXT_MUTED),
            )
            .padding(theme::S_SM)
            .into();
        }
        let editable: Vec<ActivityId> = all.iter().filter_map(|l| l.activity).collect();
        let selected_id = self.selected.and_then(|i| editable.get(i).copied());
        let mut list = column![].spacing(theme::S_HAIR);
        for (index, line) in all.iter().enumerate() {
            if line.kind != LineKind::Activity && index > 0 {
                list = list.push(Space::new().height(10));
            }
            let target = match line.activity {
                Some(id) => Target::Activity(id),
                None => Target::Category(line.name.clone()),
            };
            list = list.push(match &self.editor {
                Some(Editor::Total(edit)) if Some(edit.activity) == line.activity => {
                    self.edit_panel(edit, line.kind)
                }
                Some(Editor::Rename(rename)) if rename.target == target => {
                    self.rename_panel(rename, line.kind)
                }
                _ => self.line_view(line, selected_id.is_some() && line.activity == selected_id),
            });
        }
        if let Some(Editor::Add(add)) = &self.editor {
            list = list.push(Space::new().height(12));
            list = list.push(self.add_panel(add));
        }
        list.into()
    }

    fn line_view(&self, line: &Line, selected: bool) -> Element<'_, Message> {
        let top = line.kind != LineKind::Activity;
        let name = if top {
            text(line.name.clone()).font(theme::UI_SEMIBOLD).size(14)
        } else {
            theme::body(line.name.clone())
        };
        let name = button(name)
            .padding(Padding::from([4.0, 0.0]))
            .style(theme::button_total)
            .on_press(match line.activity {
                Some(id) => Message::Rename(id),
                None => Message::RenameCategory(line.name.clone()),
            });
        let (thickness, fill) = if top {
            (6.0, theme::LAMP_AMBER)
        } else {
            (4.0, theme::TEXT_MUTED)
        };
        let figure = text(format::duration(line.seconds))
            .font(if top { theme::MONO_BOLD } else { theme::MONO })
            .size(13)
            .width(Length::Fill)
            .align_x(Horizontal::Right);
        let figure: Element<'_, Message> = match line.activity {
            Some(id) => button(figure)
                .width(FIGURE_WIDTH)
                .height(28)
                .padding(Padding::from([6.0, 6.0]))
                .style(theme::button_total)
                .on_press(Message::EditTotal(id))
                .into(),
            None => container(figure.width(Length::Fill))
                .width(FIGURE_WIDTH)
                .padding(Padding::from([0.0, 6.0]))
                .into(),
        };
        let left = if line.kind == LineKind::Activity {
            24.0
        } else {
            8.0
        };
        container(
            row![
                container(name).width(Length::Fill),
                container(bar(line.seconds, line.bar_max, thickness, fill)).width(BAR_WIDTH),
                figure,
            ]
            .spacing(theme::S_MD)
            .align_y(Vertical::Center),
        )
        .height(if top { 36 } else { 32 })
        .align_y(Vertical::Center)
        .padding(Padding {
            top: 0.0,
            right: 2.0,
            bottom: 0.0,
            left,
        })
        .style(theme::list_row(selected))
        .into()
    }

    fn edit_panel<'a>(&'a self, edit: &'a Edit, kind: LineKind) -> Element<'a, Message> {
        let parsed = parse_edit(&edit.input).ok();
        let invalid = edit.invalid || (!edit.input.trim().is_empty() && parsed.is_none());
        let field = text_input("-30, +15, 1h 30", &edit.input)
            .id(edit_id())
            .on_input(Message::EditInput)
            .on_submit(Message::SaveEdit)
            .font(theme::MONO)
            .size(13)
            .align_x(Horizontal::Right)
            .padding(Padding::from([7.0, 8.0]))
            .width(96)
            .style(theme::inset_field(widgets::look(invalid, true)));
        let summary: Element<'_, Message> = match parsed {
            Some(parsed) => row![
                text(format::duration(edit.current))
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::TEXT_MUTED),
                theme::label(" becomes "),
                text(format::duration(preview(edit.current, parsed)))
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::LAMP_AMBER),
            ]
            .into(),
            None if invalid => theme::label("Try 1h 30, 90, +15 or -30")
                .color(theme::ALERT)
                .into(),
            None => theme::label("Type a new total, or +/- a change").into(),
        };
        let hint = keys_hint(summary);
        let panel = column![
            row![theme::body(edit.name.clone()).width(Length::Fill), field]
                .spacing(theme::S_MD)
                .align_y(Vertical::Center),
            hint,
        ];
        self.panel_frame(panel, kind)
    }

    fn rename_panel<'a>(&'a self, rename: &'a Renaming, kind: LineKind) -> Element<'a, Message> {
        let invalid = rename.invalid;
        let category = matches!(rename.target, Target::Category(_));
        let placeholder = if category {
            "Category"
        } else {
            "Category: Activity"
        };
        let field = text_input(placeholder, &rename.input)
            .id(rename_id())
            .on_input(Message::RenameInput)
            .on_submit(Message::SaveRename)
            .font(theme::UI)
            .size(14)
            .padding(Padding::from([9.0, 10.0]))
            .style(theme::inset_field(widgets::look(invalid, false)));
        let summary = match (category, invalid) {
            (true, true) => theme::label("Type a category name").color(theme::ALERT),
            (true, false) => theme::label("Same names merge"),
            (false, _) => widgets::rename_hint(rename.merge_into.is_some(), invalid),
        };
        let hint = keys_hint(summary);
        self.panel_frame(column![field, hint], kind)
    }

    /// The raised box an inline editor sits in, with any save error under it.
    fn panel_frame<'a>(
        &'a self,
        panel: iced::widget::Column<'a, Message>,
        kind: LineKind,
    ) -> Element<'a, Message> {
        let mut panel = panel.spacing(theme::S_SM);
        if let Some(error) = &self.error {
            panel = panel.push(theme::label(format!("Couldn't save: {error}")).color(theme::ALERT));
        }
        let indent = if kind == LineKind::Activity {
            16.0
        } else {
            0.0
        };
        container(
            container(panel)
                .padding(Padding {
                    top: 8.0,
                    right: 8.0,
                    bottom: 10.0,
                    left: 8.0,
                })
                .style(theme::raised_panel),
        )
        .padding(Padding {
            left: indent,
            ..Padding::ZERO
        })
        .into()
    }

    fn add_panel<'a>(&'a self, add: &'a Add) -> Element<'a, Message> {
        let bad_activity = add.invalid && ActivityName::parse(&add.activity).is_err();
        let bad_duration = add.invalid && parse_add_duration(&add.duration).is_none()
            || (!add.duration.trim().is_empty() && parse_add_duration(&add.duration).is_none());
        let activity = text_input("Category: Activity", &add.activity)
            .id(add_activity_id())
            .on_input(Message::AddActivity)
            .on_submit(Message::SaveAdd)
            .font(theme::UI)
            .size(14)
            .padding(Padding::from([9.0, 10.0]))
            .style(theme::inset_field(widgets::look(bad_activity, false)));
        let duration = text_input("e.g. 1h 30", &add.duration)
            .id(add_duration_id())
            .on_input(Message::AddDuration)
            .on_submit(Message::SaveAdd)
            .font(theme::MONO)
            .size(13)
            .align_x(Horizontal::Right)
            .padding(Padding::from([9.0, 10.0]))
            .width(104)
            .style(theme::inset_field(widgets::look(bad_duration, false)));
        let mut panel = column![
            row![activity, duration].spacing(theme::S_SM),
            theme::label(format!(
                "Counts toward {}, without a time of day.",
                format::long_day(self.correction_day(), self.today)
            )),
        ]
        .spacing(theme::S_SM);
        if let Some(error) = &self.error {
            panel = panel.push(theme::label(format!("Couldn't save: {error}")).color(theme::ALERT));
        }
        container(panel)
            .padding(10)
            .style(theme::raised_panel_active)
            .into()
    }

    fn actions(&self) -> Element<'_, Message> {
        if let Some(Editor::Add(add)) = &self.editor {
            let ready = ActivityName::parse(&add.activity).is_ok()
                && parse_add_duration(&add.duration).is_some();
            return row![
                ghost_button("Cancel", Some("esc"), Some(Message::Cancel)),
                Space::new().width(Length::Fill),
                primary_button(
                    String::from("Add"),
                    Some(icons::plus()),
                    None,
                    ready.then_some(Message::SaveAdd)
                ),
            ]
            .spacing(theme::S_XS)
            .align_y(Vertical::Center)
            .into();
        }
        let primary = if matches!(self.editor, Some(Editor::Total(_))) {
            primary_button(
                String::from("Save"),
                Some(icons::check()),
                None,
                Some(Message::SaveEdit),
            )
        } else {
            primary_button(
                String::from("Add time"),
                Some(icons::plus()),
                None,
                Some(Message::StartAdd),
            )
        };
        row![
            container(status_line(&self.status))
                .padding(Padding::from([0.0, 8.0]))
                .width(Length::Fill),
            icon_button(
                if self.paused {
                    icons::play()
                } else {
                    icons::pause()
                },
                16.0,
                Some(Message::TogglePause)
            ),
            icon_button(icons::menu(), 16.0, Some(Message::ToggleMenu)),
            primary,
        ]
        .spacing(theme::S_XS)
        .align_y(Vertical::Center)
        .into()
    }

    fn range_menu_view(&self) -> Element<'_, Message> {
        let current = current_preset(self.spec, self.today);
        let items = PRESETS.iter().map(|(preset, label)| {
            let selected = current == Some(*preset);
            let mut content = row![
                text(*label)
                    .size(14)
                    .font(if selected {
                        theme::UI_SEMIBOLD
                    } else {
                        theme::UI
                    })
                    .width(Length::Fill)
            ]
            .align_y(Vertical::Center);
            if selected {
                content = content.push(widgets::icon(icons::check(), 14.0, theme::LAMP_AMBER));
            }
            button(content)
                .width(Length::Fill)
                .height(36)
                .padding(Padding::from([8.0, 10.0]))
                .style(theme::menu_item(selected))
                .on_press(Message::Preset(*preset))
                .into()
        });
        container(column(items).spacing(theme::S_HAIR))
            .width(220)
            .padding(6)
            .style(theme::floating_menu)
            .into()
    }

    fn main_menu_view(&self) -> Element<'_, Message> {
        let item = |label: &'static str, message| {
            button(text(label).size(14))
                .width(Length::Fill)
                .height(36)
                .padding(Padding::from([8.0, 10.0]))
                .style(theme::menu_item(false))
                .on_press(message)
        };
        container(
            column![
                item("End day", Message::EndDay),
                item("Open tally", Message::OpenMain),
                item("Quit tally", Message::Quit)
            ]
            .spacing(theme::S_HAIR),
        )
        .width(180)
        .padding(6)
        .style(theme::floating_menu)
        .into()
    }
}

/// An editor's hint on the left, `enter saves · esc cancels` on the right.
fn keys_hint<'a>(summary: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        summary.into(),
        Space::new().width(Length::Fill),
        text("enter").font(theme::MONO).size(12),
        theme::label(" saves · "),
        text("esc").font(theme::MONO).size(12),
        theme::label(" cancels"),
    ]
    .align_y(Vertical::Center)
    .into()
}

#[cfg(test)]
mod tests;
