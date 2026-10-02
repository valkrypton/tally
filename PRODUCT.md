# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The author first, as a personal tool. Later published as open source for anyone who tracks their own time. Users sit at a desktop or laptop for most of the working day, switching between tasks, and want accurate timesheets without managing timers.

## Product Purpose

tally is a desktop time tracker that asks "what are you doing?" on a timer, so the user never starts or stops timers and never forgets to log. Answers build daily, weekly and monthly totals that can be corrected in place and exported.

Success: a full working week with correct daily and weekly totals, with no time lost or counted twice, and each prompt answered in a couple of seconds from the keyboard.

## Positioning

- Open source, unlike Daily, the closed-source Mac app it is modelled on.
- Works on macOS and Windows (Daily is Mac-only).
- Local only: one SQLite file on the user's machine, with no cloud and no account.
- Keyboard-first: every prompt can be answered without touching the mouse.

## Operating Context

- Lives in the system tray. Two small popups open from it: the prompt, and the dashboard of totals. A main window holds Activities, Export and Settings.
- Prompts appear only inside the user's weekly working hours, and pause while the user is away from the keyboard. On return, an away prompt asks what that time was.
- Typical use is many short interactions per day (answering a prompt), plus occasional review and correction of totals.

## Capabilities and Constraints

- The platform value above is the closest the schema allows. tally is a **native desktop app** built in Rust with **iced**, not a web app. Designs must stay within what iced can draw: no web-only effects.
- Activities have one optional level of grouping, written `Category: Activity`.
- The functional spec is `docs/superpowers/specs/2026-10-02-tally-v1-design.md`, and the core implementation plan is `docs/superpowers/plans/2026-10-03-tally-core.md`.
- Linux runs but is not polished in v1. Mobile apps and sync are not planned.

## Brand Commitments

- The product name is lowercase **tally**.
- The chosen look is the **Night shift** direction: dark and keyboard-first. It is recorded here only as a commitment; the full visual system belongs in DESIGN.md once written.
- Design canvas: https://claude.ai/artifact/P1hSRPbGfwWAKWe55ojUae (private).

## Evidence on Hand

- The functional spec, the core implementation plan and the design canvas above.
- No users, testimonials, usage data or press exist yet. Don't invent any.

## Product Principles

1. Never lose or double-count time: when unsure, ask the user rather than guess.
2. Asking should cost seconds. The prompt is pre-filled and fully usable from the keyboard.
3. The user's data stays on the user's machine.
4. Correcting is as easy as recording: totals are edited in place, not through forms.

## Accessibility & Inclusion

WCAG AA contrast (4.5:1 for body text, 3:1 for large text) and full keyboard operation of every screen.
