//! The iced daemon: owns the scheduler and the store, turns ticks, clicks and keys
//! into scheduler events, and carries out the scheduler's actions.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use iced::event::{self, Event};
use iced::widget::{Id, Space, operation};
use iced::window::{self, Level, Mode, Position};
use iced::{Color, Element, Size, Subscription, Task, keyboard, time};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use tally_core::duration::DurationEdit;
use tally_core::range::{RangeSpec, RangeUnit};
use tally_core::scheduler::{
    self, Action, Config, Event as CoreEvent, NewEntry, Scheduler, Source, State,
};
use tally_core::settings::{Settings, WeeklySchedule};
use tally_core::store::{Rename, Store};

use crate::pending::PendingWrites;
use crate::platform::events::{MenuCommand, PlatformEvent};
use crate::platform::hotkey::Hotkey;
use crate::platform::paths::Paths;
use crate::platform::tray::{Tray, TrayLook, TrayMode, TrayRect};
use crate::platform::{self, autostart, idle};
use crate::status::Status;
use crate::windows::KeyPress;
use crate::windows::away::{self, Away};
use crate::windows::dashboard::{self, Dashboard};
use crate::windows::data_error::{self, DataError};
use crate::windows::main_window::{
    self, MainWindow, Section, activities, calendar, export, settings,
};
use crate::windows::placement::{self, Edge};
use crate::windows::prompt::{self, Prompt};
use crate::windows::registry::{Registry, WindowKind};
use crate::{clock, theme};

/// How often the scheduler is ticked (spec §3.1).
pub const TICK: Duration = Duration::from_secs(30);
/// Activities offered in prompts and as away chips.
const SUGGESTIONS: usize = 50;
/// A tray click this soon after the dashboard closed itself (because the click took
/// focus away from it) must not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(400);

pub fn reopen_allowed(closed_at: Option<Instant>, now: Instant) -> bool {
    closed_at.is_none_or(|at| now.saturating_duration_since(at) > REOPEN_GUARD)
}

/// The prompt's last action wins when one `handle` call both hides and shows it
/// (e.g. returning from sleep hides the regular prompt and shows the away prompt).
pub fn final_prompt_action(actions: &[Action]) -> Option<Option<scheduler::Prompt>> {
    actions.iter().rev().find_map(|action| match action {
        Action::ShowPrompt(prompt) => Some(Some(prompt.clone())),
        Action::HidePrompt => Some(None),
        Action::Record(_) => None,
    })
}

struct Core {
    store: Store,
    scheduler: Scheduler,
    settings: Settings,
}

enum PromptWindow {
    Regular(Prompt),
    Away(Away),
}

impl PromptWindow {
    fn size(&self) -> Size {
        match self {
            PromptWindow::Regular(prompt) => prompt.window_size(),
            PromptWindow::Away(_) => away::SIZE,
        }
    }

    fn first_field(&self) -> Id {
        match self {
            PromptWindow::Regular(_) => prompt::input_id(),
            PromptWindow::Away(away) => away.first_field(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Started,
    Tick,
    Platform(PlatformEvent),
    Key(window::Id, KeyPress),
    Focused(window::Id),
    Unfocused(window::Id),
    Closed(window::Id),
    Place {
        id: window::Id,
        scale: f32,
        screen: Option<Size>,
    },
    Prompt(prompt::Message),
    Away(away::Message),
    Dashboard(dashboard::Message),
    Main(main_window::Message),
    DataError(data_error::Message),
    ExportPath(Option<PathBuf>),
}

pub struct App {
    tz: TimeZone,
    paths: Paths,
    core: Option<Core>,
    windows: Registry,
    prompt: Option<PromptWindow>,
    dashboard: Option<Dashboard>,
    main: Option<MainWindow>,
    data_error: Option<DataError>,
    tray: Option<Tray>,
    tray_rect: Option<TrayRect>,
    hotkey: Option<Hotkey>,
    hotkey_error: Option<String>,
    autostart_error: Option<String>,
    pending: PendingWrites,
    dashboard_closed_at: Option<Instant>,
    /// The last monitor size iced reported, for when it can't tell us.
    screen: Option<Size>,
    platform_started: bool,
}

impl App {
    pub fn boot(paths: Paths) -> (Self, Task<Message>) {
        let mut app = Self {
            tz: clock::system_tz(),
            paths,
            core: None,
            windows: Registry::default(),
            prompt: None,
            dashboard: None,
            main: None,
            data_error: None,
            tray: None,
            tray_rect: None,
            hotkey: None,
            hotkey_error: None,
            autostart_error: None,
            pending: PendingWrites::default(),
            dashboard_closed_at: None,
            screen: None,
            platform_started: false,
        };
        let task = match app.open_core() {
            Ok(()) => Task::none(),
            Err(error) => app.show_data_error(&error),
        };
        // `Started` runs once the event loop is up: the earliest the tray, the global
        // shortcut and the macOS activation policy can be set up.
        (app, Task::done(Message::Started).chain(task))
    }

    fn open_core(&mut self) -> anyhow::Result<()> {
        self.paths
            .create_data_dir()
            .with_context(|| format!("could not create {}", self.paths.data_dir.display()))?;
        let store = Store::open(&self.paths.database)?;
        let settings = store.load_settings()?;
        let config = Config::from_settings(&settings, self.tz.clone());
        let scheduler = match store.load_snapshot()? {
            Some(snapshot) => Scheduler::restore(config, snapshot),
            None => Scheduler::new(config, clock::now()),
        };
        tracing::info!(database = %self.paths.database.display(), "opened data file");
        self.core = Some(Core {
            store,
            scheduler,
            settings,
        });
        Ok(())
    }

    pub fn title(&self, _: window::Id) -> String {
        String::from("tally")
    }

    pub fn theme(&self, _: window::Id) -> iced::Theme {
        theme::theme()
    }

    /// Transparent, so popups can draw their own rounded frame.
    pub fn style(&self, _: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: theme::TEXT_PRIMARY,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            time::every(TICK).map(|_| Message::Tick),
            platform::events::subscription().map(Message::Platform),
            event::listen_with(route_event),
            window::close_events().map(Message::Closed),
        ])
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        match self.windows.kind(id) {
            Some(WindowKind::Prompt) => match &self.prompt {
                Some(PromptWindow::Regular(prompt)) => prompt.view().map(Message::Prompt),
                Some(PromptWindow::Away(away)) => away.view().map(Message::Away),
                None => Space::new().into(),
            },
            Some(WindowKind::Dashboard) => match &self.dashboard {
                Some(dashboard) => dashboard.view().map(Message::Dashboard),
                None => Space::new().into(),
            },
            Some(WindowKind::Main) => match &self.main {
                Some(main) => main.view().map(Message::Main),
                None => Space::new().into(),
            },
            Some(WindowKind::DataError) => match &self.data_error {
                Some(error) => error.view().map(Message::DataError),
                None => Space::new().into(),
            },
            None => Space::new().into(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Started => {
                if self.core.is_some() {
                    self.start_platform();
                    self.after_core_ready()
                } else {
                    Task::none()
                }
            }
            // `dispatch` also retries any writes that failed earlier.
            Message::Tick => self.dispatch(CoreEvent::Tick {
                idle_for: idle::idle_for(),
            }),
            Message::Platform(event) => self.platform_event(event),
            Message::Key(id, press) => self.key(id, &press),
            Message::Focused(id) => {
                self.windows.focused(id);
                Task::none()
            }
            Message::Unfocused(id) => {
                if self.windows.kind(id) == Some(WindowKind::Dashboard)
                    && self.windows.lost_focus(id)
                {
                    self.dashboard_closed_at = Some(clock::instant());
                    self.close(WindowKind::Dashboard)
                } else {
                    Task::none()
                }
            }
            Message::Closed(id) => self.closed(id),
            Message::Place { id, scale, screen } => self.place(id, scale, screen),
            Message::Prompt(message) => {
                let Some(PromptWindow::Regular(prompt)) = &mut self.prompt else {
                    return Task::none();
                };
                match prompt.update(message) {
                    Some(outcome) => self.prompt_outcome(outcome),
                    None => Task::none(),
                }
            }
            Message::Away(message) => {
                let Some(PromptWindow::Away(away)) = &mut self.prompt else {
                    return Task::none();
                };
                match away.update(message) {
                    Some(outcome) => self.away_outcome(outcome),
                    None => Task::none(),
                }
            }
            Message::Dashboard(message) => {
                let Some(dashboard) = &mut self.dashboard else {
                    return Task::none();
                };
                match dashboard.update(message) {
                    Some(outcome) => self.dashboard_outcome(outcome),
                    None => Task::none(),
                }
            }
            Message::Main(message) => {
                let Some(main) = &mut self.main else {
                    return Task::none();
                };
                match main.update(message) {
                    Some(outcome) => self.main_outcome(outcome),
                    None => Task::none(),
                }
            }
            Message::DataError(message) => self.data_error_message(message),
            Message::ExportPath(path) => {
                self.write_export(path);
                Task::none()
            }
        }
    }

    // ---- Startup ----

    /// Tray, global shortcut, Dock icon. Main thread only, after the loop starts.
    fn start_platform(&mut self) {
        if self.platform_started {
            return;
        }
        self.platform_started = true;
        match Tray::new() {
            Ok(tray) => {
                self.tray = Some(tray);
                platform::hide_dock_icon();
            }
            // Spec §7: without a tray icon, fall back to the main window.
            Err(error) => tracing::error!(error = %format!("{error:#}"), "no tray icon"),
        }
        match Hotkey::new() {
            Ok(hotkey) => self.hotkey = Some(hotkey),
            Err(error) => {
                tracing::warn!(error = %format!("{error:#}"), "no global shortcut");
                self.hotkey_error = Some(format!("{error:#}"));
            }
        }
    }

    fn after_core_ready(&mut self) -> Task<Message> {
        let Some(core) = &self.core else {
            return Task::none();
        };
        let settings = core.settings.clone();
        if let Some(hotkey) = &mut self.hotkey {
            self.hotkey_error = hotkey.set(&settings.shortcut).err();
        }
        self.autostart_error = autostart::apply(settings.start_at_login).err();
        let mut tasks = Vec::new();
        // After a restart, put the prompt that was open back on screen.
        if let Some(visible) = core.scheduler.visible_prompt().cloned() {
            tasks.push(self.show_prompt(visible, clock::now()));
        }
        if self.tray.is_none() {
            tasks.push(self.open_main());
        }
        tasks.push(Task::done(Message::Tick));
        Task::batch(tasks)
    }

    // ---- The scheduler loop ----

    /// Feeds one event to the scheduler and carries out what it asks for.
    fn dispatch(&mut self, event: CoreEvent) -> Task<Message> {
        let now = clock::now();
        let Some(core) = &mut self.core else {
            return Task::none();
        };
        let actions = core.scheduler.handle(event, now);
        for action in &actions {
            if let Action::Record(entry) = action {
                self.pending.push_entry(entry.clone());
            }
        }
        self.pending.set_snapshot(core.scheduler.snapshot());
        self.pending.flush(&mut core.store);
        let recorded = actions.iter().any(|a| matches!(a, Action::Record(_)));

        let prompt_task = match final_prompt_action(&actions) {
            Some(Some(prompt)) => self.show_prompt(prompt, now),
            Some(None) => self.close(WindowKind::Prompt),
            None => Task::none(),
        };
        if recorded
            && self
                .main
                .as_ref()
                .is_some_and(|m| m.section == Section::Calendar)
        {
            self.load_section(Section::Calendar);
        }
        let refresh_task = self.refresh(now);
        prompt_task.chain(refresh_task)
    }

    /// Brings every open window, and the tray, up to date.
    fn refresh(&mut self, now: Timestamp) -> Task<Message> {
        let Some(core) = &self.core else {
            return Task::none();
        };
        let state = core.scheduler.state().clone();
        if let Some(tray) = &mut self.tray {
            tray.show(TrayLook {
                mode: TrayMode::from_state(&state),
                failing: self.pending.is_failing(),
            });
        }
        let status = Status::of(&state, &self.tz);
        let today = self.today(now);
        if let Some(dashboard) = &mut self.dashboard {
            let totals = dashboard
                .spec()
                .resolve(&self.tz)
                .and_then(|range| core.store.totals(&range));
            match totals {
                Ok(totals) => {
                    dashboard.set_data(totals, status.clone(), Status::is_paused(&state), today)
                }
                Err(error) => tracing::warn!(%error, "could not load totals"),
            }
        }
        if let Some(main) = &mut self.main {
            main.set_status(
                status,
                main_window::pending_notice(
                    self.pending.unsaved_entries(),
                    self.pending.last_error(),
                ),
            );
        }
        // Keep the open regular prompt's times (and late notice) current.
        if let (
            Some(PromptWindow::Regular(open)),
            State::Prompting(scheduler::Prompt::Regular { due_at }),
        ) = (&self.prompt, &state)
        {
            let before = open.window_size();
            let setup = self.regular_setup(*due_at, now);
            if let Some(PromptWindow::Regular(open)) = &mut self.prompt {
                open.refresh(setup);
                let after = open.window_size();
                if after != before {
                    return self.resize_popover(WindowKind::Prompt, after);
                }
            }
        }
        Task::none()
    }

    fn today(&self, now: Timestamp) -> Date {
        now.to_zoned(self.tz.clone()).date()
    }

    // ---- Prompt ----

    fn suggestions(&self) -> Vec<tally_core::ActivityName> {
        let Some(core) = &self.core else {
            return Vec::new();
        };
        core.store.suggestions(SUGGESTIONS).unwrap_or_else(|error| {
            tracing::warn!(%error, "could not load suggestions");
            Vec::new()
        })
    }

    fn regular_setup(&self, due_at: Timestamp, now: Timestamp) -> prompt::Setup {
        let core = self.core.as_ref().expect("prompt needs the store");
        let day_end = core
            .settings
            .schedule
            .period_containing(core.scheduler.cursor(), &self.tz)
            .map(|(_, end)| end);
        prompt::Setup {
            cursor: core.scheduler.cursor(),
            due_at,
            late: core.scheduler.late_coverage(now),
            previous: core.scheduler.last_activity().cloned(),
            suggestions: self.suggestions(),
            day_end,
            tz: self.tz.clone(),
        }
    }

    fn show_prompt(&mut self, prompt: scheduler::Prompt, now: Timestamp) -> Task<Message> {
        let Some(core) = &self.core else {
            return Task::none();
        };
        // "Ask now" while the same prompt is open: keep what was typed, just refocus.
        if let (Some(PromptWindow::Regular(open)), scheduler::Prompt::Regular { due_at }) =
            (&self.prompt, &prompt)
            && open.due_at() == *due_at
            && let Some(id) = self.windows.id(WindowKind::Prompt)
        {
            return window::gain_focus(id).chain(focus_and_select(prompt::input_id()));
        }
        let window = match prompt {
            scheduler::Prompt::Regular { due_at } => {
                PromptWindow::Regular(Prompt::new(self.regular_setup(due_at, now)))
            }
            scheduler::Prompt::Away { left_at, back_at } => {
                PromptWindow::Away(Away::new(away::Setup {
                    cursor: core.scheduler.cursor(),
                    left_at,
                    back_at,
                    previous: core.scheduler.last_activity().cloned(),
                    suggestions: self.suggestions(),
                    tz: self.tz.clone(),
                }))
            }
        };
        let size = window.size();
        let field = window.first_field();
        self.prompt = Some(window);
        match self.windows.id(WindowKind::Prompt) {
            // Already on screen (e.g. "Ask now" while asking): refresh and refocus.
            Some(id) => self
                .resize_popover(WindowKind::Prompt, size)
                .chain(window::gain_focus(id))
                .chain(focus_and_select(field)),
            None => self.open_popover(WindowKind::Prompt, size),
        }
    }

    fn prompt_outcome(&mut self, outcome: prompt::Outcome) -> Task<Message> {
        let event = match outcome {
            prompt::Outcome::Answer {
                activity,
                apply_late,
            } => CoreEvent::Answered {
                activity,
                apply_late,
            },
            prompt::Outcome::Skip => CoreEvent::Skipped,
            prompt::Outcome::Snooze(duration) => CoreEvent::Snoozed(duration),
            prompt::Outcome::Keep { activity, until } => CoreEvent::KeepFor { activity, until },
            prompt::Outcome::Pause(duration) => CoreEvent::Pause(duration),
        };
        self.dispatch(event)
    }

    fn away_outcome(&mut self, outcome: away::Outcome) -> Task<Message> {
        let away::Outcome::Resolve { before, away } = outcome;
        self.dispatch(CoreEvent::AwayResolved { before, away })
    }

    // ---- Platform events ----

    fn platform_event(&mut self, event: PlatformEvent) -> Task<Message> {
        match event {
            PlatformEvent::TrayClicked(rect) => {
                self.tray_rect = Some(rect);
                self.toggle_dashboard()
            }
            PlatformEvent::Menu(MenuCommand::AskNow) => self.dispatch(CoreEvent::AskNow),
            PlatformEvent::Menu(MenuCommand::PauseOrResume) => self.toggle_pause(),
            PlatformEvent::Menu(MenuCommand::EndDay) => self.dispatch(CoreEvent::EndDay),
            PlatformEvent::Menu(MenuCommand::Open) => self.open_main(),
            PlatformEvent::Menu(MenuCommand::Quit) => self.quit(),
            PlatformEvent::Hotkey(id) => {
                if self.hotkey.as_ref().and_then(Hotkey::id) == Some(id) {
                    self.dispatch(CoreEvent::AskNow)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn toggle_pause(&mut self) -> Task<Message> {
        let paused = self
            .core
            .as_ref()
            .is_some_and(|core| Status::is_paused(core.scheduler.state()));
        self.dispatch(if paused {
            CoreEvent::Resume
        } else {
            CoreEvent::Pause(None)
        })
    }

    fn toggle_dashboard(&mut self) -> Task<Message> {
        if self.windows.is_open(WindowKind::Dashboard) {
            return self.close(WindowKind::Dashboard);
        }
        if !reopen_allowed(self.dashboard_closed_at, clock::instant()) || self.core.is_none() {
            return Task::none();
        }
        let now = clock::now();
        self.dashboard = Some(Dashboard::new(self.today(now), self.tz.clone()));
        let refresh = self.refresh(now);
        self.open_popover(WindowKind::Dashboard, dashboard::SIZE)
            .chain(refresh)
    }

    fn key(&mut self, id: window::Id, press: &KeyPress) -> Task<Message> {
        match self.windows.kind(id) {
            Some(WindowKind::Prompt) => match &mut self.prompt {
                Some(PromptWindow::Regular(prompt)) => match prompt.key(press) {
                    Some(outcome) => self.prompt_outcome(outcome),
                    None => Task::none(),
                },
                Some(PromptWindow::Away(away)) => match away.key(press) {
                    Some(outcome) => self.away_outcome(outcome),
                    None => Task::none(),
                },
                None => Task::none(),
            },
            Some(WindowKind::Dashboard) => match self.dashboard.as_mut().and_then(|d| d.key(press))
            {
                Some(outcome) => self.dashboard_outcome(outcome),
                None => Task::none(),
            },
            Some(WindowKind::Main) => match self.main.as_mut().and_then(|m| m.key(press)) {
                Some(outcome) => self.main_outcome(outcome),
                None => Task::none(),
            },
            Some(WindowKind::DataError) | None => Task::none(),
        }
    }

    // ---- Dashboard ----

    fn dashboard_outcome(&mut self, outcome: dashboard::Outcome) -> Task<Message> {
        match outcome {
            dashboard::Outcome::RangeChanged => self.refresh(clock::now()),
            dashboard::Outcome::EditTotal { activity, edit } => {
                let result = self.edit_total(|_| Ok(activity), edit);
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.saved(result);
                }
                self.refresh(clock::now())
            }
            dashboard::Outcome::AddTime { activity, seconds } => {
                let result = self.edit_total(
                    |store| store.ensure_activity(&activity),
                    DurationEdit::Add(seconds),
                );
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.saved(result);
                }
                self.refresh(clock::now())
            }
            dashboard::Outcome::Rename { id, to } => {
                let Some(core) = &mut self.core else {
                    return Task::none();
                };
                let result = match core.store.rename(id, &to) {
                    Ok(Rename::Done) => Ok(()),
                    Ok(Rename::Conflict(existing)) => {
                        if let Some(dashboard) = &mut self.dashboard {
                            dashboard.conflict(existing);
                        }
                        return Task::none();
                    }
                    Err(error) => Err(error.to_string()),
                };
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.saved(result);
                }
                self.refresh(clock::now())
            }
            dashboard::Outcome::Merge { from, into } => {
                let Some(core) = &mut self.core else {
                    return Task::none();
                };
                let result = core.store.merge(from, into).map_err(|e| e.to_string());
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.saved(result);
                }
                self.refresh(clock::now())
            }
            dashboard::Outcome::RenameCategory { from, to } => {
                let Some(core) = &mut self.core else {
                    return Task::none();
                };
                let result = core
                    .store
                    .rename_category(&from, &to)
                    .map_err(|e| e.to_string());
                if let Some(dashboard) = &mut self.dashboard {
                    dashboard.saved(result);
                }
                self.refresh(clock::now())
            }
            dashboard::Outcome::Focus(id) => operation::focus(id),
            dashboard::Outcome::TogglePause => self.toggle_pause(),
            dashboard::Outcome::EndDay => self.dispatch(CoreEvent::EndDay),
            dashboard::Outcome::OpenMain => {
                self.close(WindowKind::Dashboard).chain(self.open_main())
            }
            dashboard::Outcome::Quit => self.quit(),
            dashboard::Outcome::Close => self.close(WindowKind::Dashboard),
        }
    }

    /// Applies a dashboard edit to the activity `pick` returns, in the dashboard's range.
    fn edit_total(
        &mut self,
        pick: impl FnOnce(&mut Store) -> tally_core::Result<i64>,
        edit: DurationEdit,
    ) -> Result<(), String> {
        let (Some(core), Some(dashboard)) = (&mut self.core, &self.dashboard) else {
            return Err(String::from("tally isn't ready"));
        };
        let spec = dashboard.spec();
        let day = dashboard.correction_day();
        let result = spec.resolve(&self.tz).and_then(|range| {
            let id = pick(&mut core.store)?;
            core.store.edit_total(id, &range, day, edit)
        });
        result.map_err(|error| {
            tracing::warn!(%error, "could not edit total");
            error.to_string()
        })
    }

    // ---- Main window ----

    fn open_main(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.id(WindowKind::Main) {
            return window::gain_focus(id);
        }
        let Some(core) = &self.core else {
            return Task::none();
        };
        let mut main = MainWindow::new(
            self.today(clock::now()),
            self.tz.clone(),
            core.settings.clone(),
            self.paths.display_database(),
        );
        main.settings.set_shortcut_error(self.hotkey_error.clone());
        main.settings
            .set_autostart_error(self.autostart_error.clone());
        self.main = Some(main);
        self.load_section(Section::Calendar);
        let (id, open) = window::open(window::Settings {
            size: main_window::SIZE,
            min_size: Some(main_window::MIN_SIZE),
            position: Position::Centered,
            exit_on_close_request: true,
            ..window::Settings::default()
        });
        self.windows.insert(id, WindowKind::Main);
        let refresh = self.refresh(clock::now());
        open.then(window::gain_focus).chain(refresh)
    }

    fn load_section(&mut self, section: Section) {
        let (Some(core), Some(main)) = (&self.core, &mut self.main) else {
            return;
        };
        match section {
            Section::Activities => match core.store.activities() {
                Ok(rows) => main.activities.set_rows(rows),
                Err(error) => main.activities.saved(Err(error.to_string())),
            },
            Section::Export => {
                let today = clock::now().to_zoned(self.tz.clone()).date();
                let rows = main
                    .export
                    .spec()
                    .resolve(&self.tz)
                    .and_then(|range| core.store.export_rows(&range));
                match rows {
                    Ok(rows) => main.export.set_rows(rows, today),
                    Err(error) => main.export.saved(Err(error.to_string())),
                }
            }
            Section::Calendar => {
                let today = clock::now().to_zoned(self.tz.clone()).date();
                let logged_until = core.scheduler.cursor().min(clock::now());
                let data = calendar_data(
                    &core.store,
                    main.calendar.spec(),
                    &self.tz,
                    &core.settings.schedule,
                    logged_until,
                );
                match data {
                    Ok(data) => main.calendar.set_data(data, today),
                    Err(error) => main.calendar.saved(Err(error.to_string())),
                }
            }
            Section::Settings => {}
        }
    }

    fn main_outcome(&mut self, outcome: main_window::Outcome) -> Task<Message> {
        match outcome {
            main_window::Outcome::Opened(section) => {
                self.load_section(section);
                Task::none()
            }
            main_window::Outcome::Calendar(outcome) => self.calendar_outcome(outcome),
            main_window::Outcome::Activities(outcome) => self.activities_outcome(outcome),
            main_window::Outcome::Export(export::Outcome::RangeChanged) => {
                self.load_section(Section::Export);
                Task::none()
            }
            main_window::Outcome::Export(export::Outcome::Save { file_name }) => Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_file_name(file_name)
                    .save_file(),
                |handle| Message::ExportPath(handle.map(|h| h.path().to_path_buf())),
            ),
            main_window::Outcome::Settings(outcome) => self.settings_outcome(outcome),
        }
    }

    fn activities_outcome(&mut self, outcome: activities::Outcome) -> Task<Message> {
        let Some(core) = &mut self.core else {
            return Task::none();
        };
        let result = match outcome {
            activities::Outcome::Focus(id) => return operation::focus(id),
            activities::Outcome::Rename { id, to } => match core.store.rename(id, &to) {
                Ok(Rename::Done) => Ok(()),
                Ok(Rename::Conflict(existing)) => {
                    if let Some(main) = &mut self.main {
                        main.activities.conflict(existing);
                    }
                    return Task::none();
                }
                Err(error) => Err(error),
            },
            activities::Outcome::Merge { from, into } => core.store.merge(from, into),
            activities::Outcome::SetArchived { id, archived } => {
                core.store.set_archived(id, archived, clock::now())
            }
        };
        if let Some(main) = &mut self.main {
            main.activities
                .saved(result.map_err(|error| error.to_string()));
        }
        self.load_section(Section::Activities);
        Task::none()
    }

    fn calendar_outcome(&mut self, outcome: calendar::Outcome) -> Task<Message> {
        let Some(core) = &mut self.core else {
            return Task::none();
        };
        let result = match outcome {
            calendar::Outcome::Focus(id) => return operation::focus(id),
            calendar::Outcome::RangeChanged => {
                self.load_section(Section::Calendar);
                return Task::none();
            }
            calendar::Outcome::Rename { id, to } => match core.store.rename(id, &to) {
                Ok(Rename::Done) => Ok(()),
                Ok(Rename::Conflict(existing)) => {
                    if let Some(main) = &mut self.main {
                        main.calendar.conflict(existing);
                    }
                    return Task::none();
                }
                Err(error) => Err(error),
            },
            calendar::Outcome::Merge { from, into } => core.store.merge(from, into),
            calendar::Outcome::LogTime {
                activity,
                start,
                end,
            } => core.store.record(&NewEntry {
                activity,
                start,
                end,
                source: Source::Manual,
            }),
        };
        if let Some(main) = &mut self.main {
            main.calendar
                .saved(result.map_err(|error| error.to_string()));
        }
        self.load_section(Section::Calendar);
        // Shows a failed write in the tray; a rename also changes the dashboard.
        self.refresh(clock::now())
    }

    fn write_export(&mut self, path: Option<PathBuf>) {
        let (Some(path), Some(main)) = (path, &mut self.main) else {
            return;
        };
        let export = &main.export;
        let result = export::render(export.rows(), export.format())
            .map_err(|error| error.to_string())
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|error| error.to_string()))
            .map(|()| path.display().to_string());
        if let Err(error) = &result {
            tracing::warn!(%error, "export failed");
        }
        main.export.saved(result);
    }

    fn settings_outcome(&mut self, outcome: settings::Outcome) -> Task<Message> {
        match outcome {
            settings::Outcome::Save(settings) => self.apply_settings(settings),
            settings::Outcome::Shortcut(spec) => {
                let result = match &mut self.hotkey {
                    Some(hotkey) => hotkey.set(&spec),
                    None => Err(String::from(
                        "Global shortcuts aren't available on this system.",
                    )),
                };
                if result.is_ok() {
                    self.hotkey_error = None;
                }
                let follow_up = self
                    .main
                    .as_mut()
                    .and_then(|main| main.settings.shortcut_result(spec, result));
                match follow_up {
                    Some(settings::Outcome::Save(settings)) => self.apply_settings(settings),
                    _ => Task::none(),
                }
            }
            settings::Outcome::OpenFolder => {
                platform::reveal(&self.paths.data_dir);
                Task::none()
            }
        }
    }

    fn apply_settings(&mut self, settings: Settings) -> Task<Message> {
        let now = clock::now();
        let settings = settings.normalized();
        let Some(core) = &mut self.core else {
            return Task::none();
        };
        let saved = core
            .store
            .save_settings(&settings)
            .map_err(|error| error.to_string());
        if settings.start_at_login != core.settings.start_at_login {
            self.autostart_error = autostart::apply(settings.start_at_login).err();
        }
        core.scheduler
            .set_config(Config::from_settings(&settings, self.tz.clone()), now);
        core.settings = settings;
        self.pending.set_snapshot(core.scheduler.snapshot());
        self.pending.flush(&mut core.store);
        if let Some(main) = &mut self.main {
            main.settings.set_save_error(saved.err());
            main.settings
                .set_autostart_error(self.autostart_error.clone());
        }
        self.refresh(now)
    }

    // ---- Data-file error ----

    fn show_data_error(&mut self, error: &anyhow::Error) -> Task<Message> {
        tracing::error!(error = %format!("{error:#}"), "could not open data file");
        self.data_error = Some(DataError {
            path: self.paths.display_database(),
            error: format!("{error:#}"),
        });
        if self.windows.is_open(WindowKind::DataError) {
            return Task::none();
        }
        let (id, open) = window::open(window::Settings {
            size: data_error::SIZE,
            position: Position::Centered,
            resizable: false,
            decorations: false,
            transparent: true,
            level: Level::AlwaysOnTop,
            exit_on_close_request: false,
            ..window::Settings::default()
        });
        self.windows.insert(id, WindowKind::DataError);
        open.then(window::gain_focus)
    }

    fn data_error_message(&mut self, message: data_error::Message) -> Task<Message> {
        match message {
            data_error::Message::Retry => match self.open_core() {
                Ok(()) => {
                    self.data_error = None;
                    let close = self.close(WindowKind::DataError);
                    self.start_platform();
                    close.chain(self.after_core_ready())
                }
                Err(error) => self.show_data_error(&error),
            },
            data_error::Message::OpenFolder => {
                let folder = if self.paths.data_dir.exists() {
                    &self.paths.data_dir
                } else {
                    self.paths.data_dir.parent().unwrap_or(&self.paths.data_dir)
                };
                platform::reveal(folder);
                Task::none()
            }
            data_error::Message::Quit => iced::exit(),
        }
    }

    // ---- Windows ----

    /// Opens a hidden popover; `Place` moves it next to the tray icon and shows it.
    fn open_popover(&mut self, kind: WindowKind, size: Size) -> Task<Message> {
        let (id, open) = window::open(popover_settings(size));
        self.windows.insert(id, kind);
        open.then(move |id| {
            window::scale_factor(id).then(move |scale| {
                window::monitor_size(id).map(move |screen| Message::Place { id, scale, screen })
            })
        })
    }

    fn place(&mut self, id: window::Id, scale: f32, screen: Option<Size>) -> Task<Message> {
        let Some(kind) = self.windows.kind(id) else {
            return Task::none();
        };
        let screen = screen.or(self.screen);
        self.screen = screen;
        let size = match kind {
            WindowKind::Prompt => self.prompt.as_ref().map_or(away::SIZE, PromptWindow::size),
            _ => dashboard::SIZE,
        };
        let anchor = self
            .tray_rect
            .or_else(|| self.tray.as_ref().and_then(Tray::rect));
        let origin = placement::popover_origin(anchor, scale, size, screen, Edge::for_platform());
        let shown = window::move_to(id, origin)
            .chain(window::set_mode(id, Mode::Windowed))
            .chain(window::gain_focus(id));
        match (kind, &self.prompt) {
            (WindowKind::Prompt, Some(prompt)) => {
                shown.chain(focus_and_select(prompt.first_field()))
            }
            _ => shown,
        }
    }

    /// Resizes a popover and places it again, so a prompt that grows stays inside the
    /// screen and attached to the tray.
    fn resize_popover(&mut self, kind: WindowKind, size: Size) -> Task<Message> {
        let Some(id) = self.windows.id(kind) else {
            return Task::none();
        };
        window::resize(id, size).chain(window::scale_factor(id).then(move |scale| {
            window::monitor_size(id).map(move |screen| Message::Place { id, scale, screen })
        }))
    }

    fn close(&mut self, kind: WindowKind) -> Task<Message> {
        let Some(id) = self.windows.id(kind) else {
            return Task::none();
        };
        self.windows.remove(id);
        self.forget(kind);
        window::close(id)
    }

    fn forget(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Prompt => self.prompt = None,
            WindowKind::Dashboard => self.dashboard = None,
            WindowKind::Main => self.main = None,
            WindowKind::DataError => self.data_error = None,
        }
    }

    /// A window closed by the OS or the user (our own closes were already forgotten).
    fn closed(&mut self, id: window::Id) -> Task<Message> {
        let Some(kind) = self.windows.remove(id) else {
            return Task::none();
        };
        self.forget(kind);
        match kind {
            // Without a tray icon the main window is the only way back in.
            WindowKind::Main if self.tray.is_none() => self.quit(),
            WindowKind::DataError if self.core.is_none() => iced::exit(),
            // A prompt closed behind our back counts as skipped, so the scheduler
            // doesn't wait for an answer that can't come.
            WindowKind::Prompt => self.dispatch(CoreEvent::Skipped),
            _ => Task::none(),
        }
    }

    fn quit(&mut self) -> Task<Message> {
        if let Some(core) = &mut self.core {
            self.pending.set_snapshot(core.scheduler.snapshot());
            self.pending.flush(&mut core.store);
            if self.pending.is_failing() {
                tracing::error!(
                    unsaved = self.pending.unsaved_entries(),
                    "quitting with unsaved entries"
                );
            }
        }
        iced::exit()
    }
}

/// Everything the Calendar draws for `spec`: the entries and a totals row for each day.
fn calendar_data(
    store: &Store,
    spec: RangeSpec,
    tz: &TimeZone,
    schedule: &WeeklySchedule,
    logged_until: Timestamp,
) -> tally_core::Result<calendar::CalendarData> {
    let range = spec.resolve(tz)?;
    let entries = store.entries(&range)?;
    // ponytail: one totals query per day (<=42); one grouped query if months get slow
    let days = range
        .first_day
        .series(jiff::ToSpan::days(1))
        .take_while(|day| *day <= range.last_day)
        .map(|day| {
            Ok((
                day,
                store.totals(&RangeSpec::new(RangeUnit::Day, day).resolve(tz)?)?,
            ))
        })
        .collect::<tally_core::Result<_>>()?;
    Ok(calendar::CalendarData {
        entries,
        days,
        schedule: schedule.clone(),
        logged_until,
    })
}

fn popover_settings(size: Size) -> window::Settings {
    window::Settings {
        size,
        position: Position::Default,
        visible: false,
        resizable: false,
        decorations: false,
        transparent: true,
        level: Level::AlwaysOnTop,
        minimizable: false,
        exit_on_close_request: false,
        #[cfg(target_os = "windows")]
        platform_specific: window::settings::PlatformSpecific {
            skip_taskbar: true,
            undecorated_shadow: false,
            ..Default::default()
        },
        ..window::Settings::default()
    }
}

fn focus_and_select(id: Id) -> Task<Message> {
    operation::focus(id.clone()).chain(operation::select_all(id))
}

/// Keys go to the window they were typed in, even when a field consumed them;
/// a lost focus closes the dashboard.
fn route_event(event: Event, status: event::Status, id: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            physical_key,
            ..
        }) => Some(Message::Key(
            id,
            KeyPress {
                key,
                modifiers,
                physical: physical_key,
                captured: status == event::Status::Captured,
            },
        )),
        Event::Window(window::Event::Focused) => Some(Message::Focused(id)),
        Event::Window(window::Event::Unfocused) => Some(Message::Unfocused(id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
