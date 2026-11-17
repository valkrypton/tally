//! Activities: rename (merging on conflict), merge into, archive and restore, search.
//! Artboard B-Activities.

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{
    Id, Space, button, checkbox, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Element, Length, Padding};
use tally_core::ActivityName;
use tally_core::store::{ActivityId, ActivityRow};

use crate::windows::KeyPress;
use crate::windows::widgets;
use crate::{format, theme};

const TOTAL_WIDTH: f32 = 96.0;
const ACTIONS_WIDTH: f32 = 220.0;

pub fn search_id() -> Id {
    Id::new("activities-search")
}

pub fn editor_id() -> Id {
    Id::new("activities-editor")
}

/// A category heading and its activities, after search and the archived filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupView<'a> {
    /// `None` is "No category", listed last.
    pub category: Option<String>,
    pub seconds: i64,
    pub rows: Vec<&'a ActivityRow>,
}

pub fn groups<'a>(
    rows: &'a [ActivityRow],
    search: &str,
    show_archived: bool,
) -> Vec<GroupView<'a>> {
    let needle = search.trim().to_lowercase();
    let mut groups: Vec<GroupView<'a>> = Vec::new();
    for row in rows {
        if row.archived && !show_archived {
            continue;
        }
        if !needle.is_empty() && !row.name.to_string().to_lowercase().contains(&needle) {
            continue;
        }
        match groups.iter_mut().find(|g| g.category == row.name.category) {
            Some(group) => {
                group.seconds += row.total_seconds;
                group.rows.push(row);
            }
            None => groups.push(GroupView {
                category: row.name.category.clone(),
                seconds: row.total_seconds,
                rows: vec![row],
            }),
        }
    }
    // Named categories alphabetically (the store's order), "No category" last.
    groups.sort_by_key(|g| g.category.is_none());
    groups
}

/// The activity with exactly this name, ignoring case (the store's own rule).
pub fn find_by_name(rows: &[ActivityRow], name: &ActivityName) -> Option<ActivityId> {
    let wanted = name.to_string().to_lowercase();
    rows.iter()
        .find(|r| r.name.to_string().to_lowercase() == wanted)
        .map(|r| r.id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Rename,
    MergeInto,
}

#[derive(Debug, Clone)]
struct Editor {
    id: ActivityId,
    mode: Mode,
    input: String,
    /// Why the input can't be used, shown under it.
    problem: Option<String>,
    /// Waiting for "Merge" to be confirmed.
    merge_into: Option<ActivityId>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    ShowArchived(bool),
    Hover(Option<ActivityId>),
    Rename(ActivityId),
    MergeInto(ActivityId),
    EditorInput(String),
    SubmitEditor,
    ConfirmMerge,
    Cancel,
    Archive(ActivityId),
    Restore(ActivityId),
    RenameSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Rename { id: ActivityId, to: ActivityName },
    Merge { from: ActivityId, into: ActivityId },
    SetArchived { id: ActivityId, archived: bool },
    Focus(Id),
}

#[derive(Debug, Clone, Default)]
pub struct Activities {
    rows: Vec<ActivityRow>,
    search: String,
    show_archived: bool,
    hovered: Option<ActivityId>,
    selected: Option<ActivityId>,
    editor: Option<Editor>,
    error: Option<String>,
}

impl Activities {
    pub fn set_rows(&mut self, rows: Vec<ActivityRow>) {
        self.rows = rows;
        if let Some(editor) = &self.editor
            && !self.rows.iter().any(|r| r.id == editor.id)
        {
            self.editor = None;
        }
    }

    /// The store said a rename collides with `existing`: offer to merge instead.
    pub fn conflict(&mut self, existing: ActivityId) {
        if let Some(editor) = &mut self.editor {
            editor.merge_into = Some(existing);
        }
    }

    /// Result of a rename, merge or archive. Errors keep the editor open.
    pub fn saved(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.editor = None;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn visible_ids(&self) -> Vec<ActivityId> {
        groups(&self.rows, &self.search, self.show_archived)
            .iter()
            .flat_map(|g| g.rows.iter().map(|r| r.id))
            .collect()
    }

    fn row(&self, id: ActivityId) -> Option<&ActivityRow> {
        self.rows.iter().find(|r| r.id == id)
    }

    fn open_editor(&mut self, id: ActivityId, mode: Mode) -> Option<Outcome> {
        let row = self.row(id)?;
        let input = match mode {
            Mode::Rename => row.name.to_string(),
            Mode::MergeInto => String::new(),
        };
        self.editor = Some(Editor {
            id,
            mode,
            input,
            problem: None,
            merge_into: None,
        });
        self.error = None;
        self.selected = Some(id);
        Some(Outcome::Focus(editor_id()))
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Search(value) => {
                self.search = value;
                None
            }
            Message::ShowArchived(on) => {
                self.show_archived = on;
                None
            }
            Message::Hover(id) => {
                self.hovered = id;
                None
            }
            Message::Rename(id) => self.open_editor(id, Mode::Rename),
            Message::MergeInto(id) => self.open_editor(id, Mode::MergeInto),
            Message::RenameSelected => {
                let id = self.selected?;
                self.open_editor(id, Mode::Rename)
            }
            Message::EditorInput(value) => {
                let editor = self.editor.as_mut()?;
                editor.input = value;
                editor.problem = None;
                editor.merge_into = None;
                None
            }
            Message::SubmitEditor => {
                let editor = self.editor.as_ref()?;
                if editor.merge_into.is_some() {
                    return self.update(Message::ConfirmMerge);
                }
                let (id, mode) = (editor.id, editor.mode);
                let Ok(name) = ActivityName::parse(&editor.input) else {
                    self.set_problem("Type a name, or Category: Name.");
                    return None;
                };
                match mode {
                    Mode::Rename => Some(Outcome::Rename { id, to: name }),
                    Mode::MergeInto => match find_by_name(&self.rows, &name) {
                        Some(target) if target != id => {
                            if let Some(editor) = &mut self.editor {
                                editor.merge_into = Some(target);
                            }
                            None
                        }
                        Some(_) => {
                            self.set_problem("That's this activity. Pick another one.");
                            None
                        }
                        None => {
                            self.set_problem(&format!("There's no activity called {name}."));
                            None
                        }
                    },
                }
            }
            Message::ConfirmMerge => {
                let editor = self.editor.as_ref()?;
                Some(Outcome::Merge {
                    from: editor.id,
                    into: editor.merge_into?,
                })
            }
            Message::Cancel => {
                self.editor = None;
                self.error = None;
                None
            }
            Message::Archive(id) => Some(Outcome::SetArchived { id, archived: true }),
            Message::Restore(id) => Some(Outcome::SetArchived {
                id,
                archived: false,
            }),
        }
    }

    fn set_problem(&mut self, problem: &str) {
        if let Some(editor) = &mut self.editor {
            editor.problem = Some(problem.to_owned());
        }
    }

    /// ↑/↓ choose a row, Enter renames it, ⌘⌫ archives or restores it, ⌘F searches,
    /// Esc cancels the editor.
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        let command = press.modifiers.command();
        match press.key.as_ref() {
            Key::Named(Named::Escape) => self.update(Message::Cancel),
            Key::Named(Named::ArrowUp | Named::ArrowDown) if self.editor.is_none() => {
                let ids = self.visible_ids();
                let current = self
                    .selected
                    .and_then(|id| ids.iter().position(|i| *i == id));
                let next = match (current, press.is(Named::ArrowUp)) {
                    (None, _) => 0,
                    (Some(i), true) => i.saturating_sub(1),
                    (Some(i), false) => (i + 1).min(ids.len().saturating_sub(1)),
                };
                self.selected = ids.get(next).copied();
                None
            }
            Key::Named(Named::Enter) if !press.captured && self.editor.is_none() => {
                self.update(Message::RenameSelected)
            }
            Key::Named(Named::Backspace | Named::Delete) if command && self.editor.is_none() => {
                let row = self.row(self.selected?)?;
                Some(Outcome::SetArchived {
                    id: row.id,
                    archived: !row.archived,
                })
            }
            Key::Character(c) if command && c.eq_ignore_ascii_case("f") => {
                Some(Outcome::Focus(search_id()))
            }
            _ => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = row![
            theme::headline("Activities").width(Length::Fill),
            text_input("Search", &self.search)
                .id(search_id())
                .on_input(Message::Search)
                .on_submit(Message::RenameSelected)
                .font(theme::UI)
                .size(13)
                .padding(Padding::from([9.0, 10.0]))
                .width(200)
                .style(theme::field(theme::FieldLook::Resting)),
            checkbox(self.show_archived)
                .label("Show archived")
                .on_toggle(Message::ShowArchived)
                .size(16)
                .text_size(13)
                .font(theme::UI)
                .style(theme::checkbox_style),
        ]
        .spacing(theme::S_MD)
        .align_y(Vertical::Center);

        let columns = container(
            row![
                theme::label("Name").width(Length::Fill),
                theme::label("All time")
                    .width(TOTAL_WIDTH)
                    .align_x(Horizontal::Right),
                Space::new().width(ACTIONS_WIDTH),
            ]
            .spacing(theme::S_LG),
        )
        .padding(Padding {
            top: 0.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        });

        let mut list = column![].spacing(theme::S_HAIR);
        let all = groups(&self.rows, &self.search, self.show_archived);
        if all.is_empty() {
            list = list.push(
                container(
                    theme::small(if self.rows.is_empty() {
                        "Nothing here yet. Activities appear once you answer a prompt."
                    } else {
                        "No activities match."
                    })
                    .color(theme::TEXT_MUTED),
                )
                .padding(12),
            );
        }
        for (index, group) in all.iter().enumerate() {
            list = list.push(Space::new().height(if index == 0 { 8 } else { 12 }));
            list = list.push(self.category_view(group));
            for activity in &group.rows {
                list = list.push(match &self.editor {
                    Some(editor) if editor.id == activity.id => self.editor_view(editor, activity),
                    _ => self.row_view(activity),
                });
            }
        }

        let mut content =
            column![header, column![columns, widgets::divider(), list]].spacing(theme::S_LG);
        if let Some(error) = &self.error {
            content =
                content.push(theme::label(format!("Couldn't save: {error}")).color(theme::ALERT));
        }
        content = content.push(
            row![
                theme::label("To move an activity to another category, rename it with the new prefix, for example "),
                text("Study: Email").font(theme::MONO).size(12),
                theme::label("."),
            ]
            .wrap(),
        );
        scrollable(content)
            .height(Length::Fill)
            .style(theme::scroll)
            .into()
    }

    fn category_view(&self, group: &GroupView<'_>) -> Element<'_, Message> {
        let (name, color) = match &group.category {
            Some(category) => (category.clone(), theme::TEXT_PRIMARY),
            None => (String::from("No category"), theme::TEXT_MUTED),
        };
        let total: Element<'_, Message> = if group.category.is_some() {
            text(format::duration(group.seconds))
                .font(theme::MONO)
                .size(12)
                .color(theme::TEXT_MUTED)
                .width(TOTAL_WIDTH)
                .align_x(Horizontal::Right)
                .into()
        } else {
            Space::new().width(TOTAL_WIDTH).into()
        };
        container(
            row![
                text(name)
                    .font(theme::UI_SEMIBOLD)
                    .size(13)
                    .color(color)
                    .width(Length::Fill),
                total,
                Space::new().width(ACTIONS_WIDTH),
            ]
            .spacing(theme::S_LG)
            .align_y(Vertical::Center),
        )
        .height(32)
        .align_y(Vertical::Center)
        .padding(Padding::from([0.0, 12.0]))
        .into()
    }

    fn row_view<'a>(&'a self, activity: &'a ActivityRow) -> Element<'a, Message> {
        let id = activity.id;
        let active = self.hovered == Some(id) || self.selected == Some(id);
        let small_button = |label: &'a str, message: Message| {
            button(theme::small(label))
                .height(30)
                .padding(Padding::from([7.0, 10.0]))
                .style(theme::button_ghost)
                .on_press(message)
        };
        let actions: Element<'a, Message> = if activity.archived {
            row![
                Space::new().width(Length::Fill),
                small_button("Restore", Message::Restore(id))
            ]
            .into()
        } else if active {
            row![
                Space::new().width(Length::Fill),
                small_button("Rename", Message::Rename(id)),
                small_button("Merge into…", Message::MergeInto(id)),
                small_button("Archive", Message::Archive(id)),
            ]
            .spacing(theme::S_XS)
            .into()
        } else {
            Space::new().width(Length::Fill).into()
        };
        let color = if activity.archived {
            theme::TEXT_MUTED
        } else {
            theme::TEXT_PRIMARY
        };
        let mut name = row![theme::body(activity.name.name.clone()).color(color)]
            .spacing(theme::S_SM)
            .align_y(Vertical::Center);
        if activity.archived {
            name = name.push(
                container(text("archived").size(11).color(theme::TEXT_MUTED))
                    .padding(Padding::from([1.0, 6.0]))
                    .style(theme::key_cap),
            );
        }
        let line = container(
            row![
                container(name).width(Length::Fill),
                text(format::duration(activity.total_seconds))
                    .font(theme::MONO)
                    .size(13)
                    .color(color)
                    .width(TOTAL_WIDTH)
                    .align_x(Horizontal::Right),
                container(actions).width(ACTIONS_WIDTH),
            ]
            .spacing(theme::S_LG)
            .align_y(Vertical::Center),
        )
        .height(40)
        .align_y(Vertical::Center)
        .padding(Padding {
            top: 5.0,
            right: 12.0,
            bottom: 5.0,
            left: 28.0,
        })
        .style(theme::list_row(active));
        mouse_area(line)
            .on_enter(Message::Hover(Some(id)))
            .on_exit(Message::Hover(None))
            .into()
    }

    fn editor_view<'a>(
        &'a self,
        editor: &'a Editor,
        activity: &'a ActivityRow,
    ) -> Element<'a, Message> {
        let placeholder = match editor.mode {
            Mode::Rename => "New name",
            Mode::MergeInto => "Activity to merge into",
        };
        let input = text_input(placeholder, &editor.input)
            .id(editor_id())
            .on_input(Message::EditorInput)
            .on_submit(Message::SubmitEditor)
            .font(theme::UI)
            .size(14)
            .padding(Padding::from([8.0, 10.0]))
            .style(theme::inset_field(widgets::look(
                editor.problem.is_some(),
                false,
            )));
        let mut panel = column![
            row![
                container(input).width(Length::Fill),
                text(format::duration(activity.total_seconds))
                    .font(theme::MONO)
                    .size(13)
                    .width(TOTAL_WIDTH)
                    .align_x(Horizontal::Right),
                Space::new().width(ACTIONS_WIDTH),
            ]
            .spacing(theme::S_LG)
            .align_y(Vertical::Center),
        ]
        .spacing(10);
        let total = format::duration(activity.total_seconds);
        let (sentence, primary): (Option<String>, Option<(&str, Message)>) = match (
            editor.merge_into.and_then(|id| self.row(id)),
            &editor.problem,
        ) {
            (Some(target), _) if editor.mode == Mode::Rename => (
                Some(format!(
                    "{} already exists. Merge these {total} into it?",
                    target.name
                )),
                Some(("Merge", Message::ConfirmMerge)),
            ),
            (Some(target), _) => (
                Some(format!(
                    "Merge {total} of {} into {}?",
                    activity.name.name, target.name
                )),
                Some(("Merge", Message::ConfirmMerge)),
            ),
            (None, Some(problem)) => (Some(problem.clone()), None),
            (None, None) => (None, None),
        };
        let mut actions = row![].spacing(theme::S_MD).align_y(Vertical::Center);
        actions = actions.push(match (&sentence, &editor.problem) {
            (Some(text_line), Some(_)) => theme::small(text_line.clone())
                .color(theme::ALERT)
                .width(Length::Fill),
            (Some(text_line), None) => theme::small(text_line.clone())
                .color(theme::LAMP_AMBER_INK)
                .width(Length::Fill),
            (None, _) => theme::label("enter saves · esc cancels").width(Length::Fill),
        });
        actions = actions.push(
            button(theme::small("Cancel"))
                .height(32)
                .padding(Padding::from([8.0, 12.0]))
                .style(theme::button_ghost)
                .on_press(Message::Cancel),
        );
        let (label, message) = primary.unwrap_or(match editor.mode {
            Mode::Rename => ("Rename", Message::SubmitEditor),
            Mode::MergeInto => ("Find", Message::SubmitEditor),
        });
        actions = actions.push(
            button(text(label).font(theme::UI_SEMIBOLD).size(13))
                .height(32)
                .padding(Padding::from([8.0, 14.0]))
                .style(theme::button_primary)
                .on_press(message),
        );
        panel = panel.push(actions);
        container(panel)
            .padding(Padding {
                top: 10.0,
                right: 12.0,
                bottom: 12.0,
                left: 28.0,
            })
            .style(theme::raised_panel_active)
            .into()
    }
}

#[cfg(test)]
mod tests;
