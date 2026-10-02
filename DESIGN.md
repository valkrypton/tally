---
name: tally
description: A quiet, keyboard-first desktop time tracker in the Night shift look.
colors:
  lamp-amber: "#FFB23E"
  lamp-amber-hover: "#FFC86E"
  lamp-amber-wash: "#241E14"
  lamp-amber-ink: "#F5E6CC"
  lamp-amber-ink-muted: "#C9B48F"
  console-night: "#121417"
  console-deep: "#0E1012"
  console-raised: "#1A1D21"
  console-hover: "#1E2226"
  console-floating: "#22262B"
  console-track: "#23272C"
  console-selected: "#2C3137"
  hairline: "#262A2F"
  outline: "#343A40"
  text-primary: "#E8EAED"
  text-muted: "#A3AAB2"
  text-placeholder: "#8A9199"
  text-disabled: "#4A5058"
  status-tracking: "#7CE38B"
  alert: "#FF8A80"
  alert-wash: "#2B1A1A"
typography:
  headline:
    fontFamily: "Schibsted Grotesk, Helvetica Neue, sans-serif"
    fontSize: "20px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Schibsted Grotesk, Helvetica Neue, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Schibsted Grotesk, Helvetica Neue, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.4
  body-small:
    fontFamily: "Schibsted Grotesk, Helvetica Neue, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Schibsted Grotesk, Helvetica Neue, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.4
  figure-display:
    fontFamily: "JetBrains Mono, Menlo, monospace"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.03em"
  figure:
    fontFamily: "JetBrains Mono, Menlo, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.4
    fontFeature: "tnum"
  key:
    fontFamily: "JetBrains Mono, Menlo, monospace"
    fontSize: "11px"
    fontWeight: 400
    lineHeight: 1.3
rounded:
  bar: "3px"
  key: "4px"
  control: "6px"
  field: "8px"
  menu: "10px"
  main-window: "12px"
  popup: "14px"
spacing:
  hair: "2px"
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "20px"
  xxl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.lamp-amber}"
    textColor: "{colors.console-night}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    height: "40px"
  button-primary-hover:
    backgroundColor: "{colors.lamp-amber-hover}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.text-primary}"
    typography: "{typography.body-small}"
    rounded: "{rounded.control}"
    padding: "0 10px"
    height: "40px"
  button-ghost-hover:
    backgroundColor: "{colors.console-hover}"
  input-prompt:
    backgroundColor: "{colors.console-raised}"
    textColor: "{colors.text-primary}"
    typography: "{typography.body}"
    rounded: "{rounded.field}"
    padding: "0 12px"
    height: "48px"
  segmented-selected:
    backgroundColor: "{colors.console-selected}"
    textColor: "{colors.text-primary}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    height: "30px"
  list-row-selected:
    backgroundColor: "{colors.console-hover}"
    textColor: "{colors.text-primary}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "8px"
  chip-selected:
    backgroundColor: "{colors.lamp-amber-wash}"
    textColor: "{colors.lamp-amber-ink}"
    typography: "{typography.body-small}"
    rounded: "{rounded.control}"
    height: "32px"
  key-cap:
    backgroundColor: "transparent"
    textColor: "{colors.text-muted}"
    typography: "{typography.key}"
    rounded: "{rounded.key}"
    padding: "1px 5px"
---

# Design System: tally

## Overview

**Creative North Star: "The Quiet Console"**

tally is a keyboard-first console that stays out of the way until it asks one thing. Most of the time it is a dark, still surface with one warm light on it: the amber of whatever needs your answer right now. Nothing else competes. The mood is calm and warm, dark without being cold or clinical.

Density is moderate and even. Popups are small (400px wide) and read top to bottom: a question, an answer field, a short list, and a footer of actions with their keys shown. Numbers are the content, so times and durations get a monospace face and aligned figures, while everything else is set in a plain grotesk.

Everything here must be drawable in iced: solid fills, 1px borders, rounded rectangles and two bundled fonts. There are no gradients, blurs, shadows or web-only effects.

**Key Characteristics:**
- Dark graphite surfaces separated by tone, not shadow.
- One accent, Lamp Amber, reserved for the current answer, the main action and focus.
- Monospace only for figures, times and key labels.
- Every action shows its key.
- Completely flat.

## Colors

A narrow graphite ramp from near-black to slate, lit by a single warm amber, with green and red used only to report state.

### Primary
- **Lamp Amber** (`lamp-amber`): the one warm light. Fills the main action in each window, outlines the field or row being answered or edited, colours category bars and today's total, and is the focus ring. Its hover state is **Lamp Amber, raised** (`lamp-amber-hover`).
- **Amber Wash** (`lamp-amber-wash`): a dark amber-tinted surface for selected chips and the late-answer notice, with **Amber Ink** (`lamp-amber-ink`) for its text and **Amber Ink, muted** (`lamp-amber-ink-muted`) for its secondary line.

### Neutral
- **Console Night** (`console-night`): the base background of every window, and the text colour on amber fills.
- **Console Deep** (`console-deep`): the main window's sidebar and the export preview, one step darker than the base.
- **Console Raised** (`console-raised`): fields, segmented-control tracks, edit panels and the active category of a menu.
- **Console Hover** (`console-hover`): hover on ghost buttons and the selected row in a list.
- **Console Floating** (`console-floating`): menus that open over other content.
- **Console Track** (`console-track`): the empty part of a progress bar.
- **Console Selected** (`console-selected`): the chosen option in a segmented control or menu.
- **Hairline** (`hairline`): dividers and window edges.
- **Outline** (`outline`): borders of resting fields, key caps and floating menus.
- **Text Primary** (`text-primary`), **Text Muted** (`text-muted`), **Text Placeholder** (`text-placeholder`) and **Text Disabled** (`text-disabled`): the four text levels. Muted is the secondary text on every surface; placeholder is for empty fields only; disabled is for controls that can't be used yet.

### State
- **Tracking Green** (`status-tracking`): only the small dot that says tracking is running.
- **Alert Red** (`alert`) on **Alert Wash** (`alert-wash`): errors only, such as the data-file error.

### Named Rules
**The One Lamp Rule.** Each window has at most one amber-filled control. Amber marks what needs you now; if two things are amber, neither is.

**The State Colour Rule.** Green and red report state and are never decoration. Green is only the tracking dot; red is only errors.

## Typography

**UI Font:** Schibsted Grotesk (with Helvetica Neue, sans-serif)
**Figure and Key Font:** JetBrains Mono (with Menlo, monospace)

**Character:** a plain, slightly warm grotesk for words, and a clear monospace for numbers, so a column of durations lines up and reads as data.

### Hierarchy
- **Headline** (700, 20px, 1.2): main-window section titles such as "Activities".
- **Title** (600, 15px, 1.3): popup questions such as "What were you doing?" and "Welcome back".
- **Body** (400, 14px, 1.4): activity names, menu items and field text.
- **Body small** (400, 13px, 1.45): button labels, notes and descriptions.
- **Label** (400, 12px, 1.4): secondary details, section labels and segmented options.
- **Figure display** (700, 28px, 1.1): the tracked total at the top of the dashboard.
- **Figure** (400 or 700, 13px, tabular): durations, times and dates in lists.
- **Key** (400, 11px): key caps such as `enter`, `esc` and `⌘K`.

### Named Rules
**The Figures Rule.** Monospace is for numbers, times and keys only. Words are never set in monospace to look technical.

**The Duration Rule.** Durations are always written `6h 35m`, never `6:35`, so they can't be mistaken for a time of day.

**The 11px Floor.** No text is smaller than 11px, and only key caps use 11px.

## Layout

- **Popups** are 400px wide: the prompt is 340px tall and the dashboard 620px. Content sits 12–16px from the edges.
- **The main window** is 880 × 640px: a 200px sidebar, then content with 24px side padding.
- **Spacing** runs 2, 4, 8, 12, 16, 20 and 24px. Related items sit 2–8px apart and separate groups 12–24px apart.
- **Lists** use 40px rows with 8px padding.
- **Number columns** are right-aligned in fixed-width tracks, so figures line up down the list.

**The Footer Rule.** Every popup ends in a footer bar above a hairline: secondary actions on the left, the one primary action on the right.

## Elevation & Depth

tally is completely flat. There are no shadows anywhere. Depth comes only from tone: each layer is a slightly lighter graphite than the one under it (deep, then night, then raised, then floating). Anything that floats over other content, such as a menu, separates by its lighter **Console Floating** surface plus a 1px **Outline** border.

**The No-Shadow Rule.** Never add a shadow. If something needs to stand out, give it a lighter tone or a hairline border.

## Shapes

Rounded rectangles throughout, with corners that grow with the element's size:
- bars (3px)
- key caps (4px)
- buttons, rows and small fields (6px)
- large fields and panels (8px)
- menus (10px)
- the main window (12px)
- popups (14px)

Borders are always 1px. Pills appear only for the small quick-pick chips in the away prompt.

## Components

### Buttons
Quiet until touched.
- **Shape:** gently rounded (6px), 40px tall in footers.
- **Primary:** Lamp Amber fill with Console Night text, semibold, 14px side padding. When it carries a key cap, the cap sits on a faint dark tint inside the button. It turns Lamp Amber, raised on hover.
- **Ghost:** no fill and primary text, with 10px side padding. It takes the Console Hover fill on hover. Footer ghost buttons show their key cap after the label.
- **Disabled:** dark amber wash with muted amber ink, and no hover change.
- **Focus:** a 2px Lamp Amber ring, 2px outside the control.

### Inputs / Fields
- **Style:** Console Raised fill, 1px Outline border, 8px corners. The prompt's main field is 48px tall with 16px text.
- **Active:** the border turns Lamp Amber and the caret is amber. A field being answered or edited is always outlined in amber.
- **Placeholder / Disabled:** placeholder text uses Text Placeholder. Disabled fields drop their fill, use a Console Track border and Text Disabled text.

### Lists and Rows
- **Style:** 40px rows with 8px padding and 6px corners. The selected row takes Console Hover.
- **Number keys:** recent activities carry 20px key chips. The selected one is filled amber with dark text; the rest are outlined.
- **Category rows:** semibold names with a 6px amber bar. Activity rows are indented 16px, with a 4px muted bar.

### Segmented Control
A Console Raised track with 3px padding and 2px gaps. The chosen option takes Console Selected with semibold primary text; the others are muted until hovered.

### Chips
- **Unselected:** an Outline border with no fill.
- **Selected:** Amber Wash fill, Lamp Amber border and Amber Ink text.
- **"Don't count it":** a dashed border, so it reads as the option of not logging.

### Menus
Console Floating surface with a 1px Outline border and 10px corners. Options are grouped under short muted labels, as small buttons in a grid. The current option takes Console Selected.

### Navigation
The main-window sidebar uses Console Deep, 36px items and 6px corners. The current section takes Console Hover with semibold text; the others are muted and brighten on hover. A tracking status line sits at the bottom of the sidebar.

### Prompt (signature component)
The heart of tally:
- the question as a Title, with its time window in figures on the right
- the amber-outlined answer field, filled in with the last answer
- up to four recent activities with number keys
- a footer with Skip `esc`, More `⌘K` and the amber primary action `enter`

The late, new-activity and away versions keep this shape and add one element each.

## Do's and Don'ts

### Do:
- **Do** keep exactly one amber-filled control per window (the One Lamp Rule).
- **Do** show the key next to every action that has one.
- **Do** write durations as `2h 40m`, and right-align them in tabular figures.
- **Do** separate layers by tone, from Console Deep up to Console Floating, with 1px borders when needed.
- **Do** keep text at 4.5:1 contrast or better. Primary and muted text pass on every surface; placeholder text passes only on Console Hover or darker, so use it only inside fields.
- **Do** stay within what iced draws natively: solid fills, 1px borders, rounded corners, and the two bundled fonts.

### Don't:
- **Don't** use shadows, gradients, blur or glow.
- **Don't** set words in monospace to make them look technical.
- **Don't** use green or red for anything except tracking status and errors.
- **Don't** write durations like clock times (`6:35`).
- **Don't** draw bars, icons or charts with text characters. Draw them.
- **Don't** go below 11px for any text.
