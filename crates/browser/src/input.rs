//! Input Handler
//!
//! Converts GPUI input events to CEF input events for browser interaction.

use crate::keycodes::{key_name_to_windows_vk, macos_keycode_to_windows_vk};
use crate::tab::BrowserTab;
use cef::{KeyEvent, KeyEventType, MouseButtonType};
use gpui::{
    Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollDelta, ScrollWheelEvent,
};

const EVENTFLAG_SHIFT_DOWN: u32 = 1 << 1;
const EVENTFLAG_CONTROL_DOWN: u32 = 1 << 2;
const EVENTFLAG_ALT_DOWN: u32 = 1 << 3;
const EVENTFLAG_LEFT_MOUSE_BUTTON: u32 = 1 << 4;
const EVENTFLAG_MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
const EVENTFLAG_RIGHT_MOUSE_BUTTON: u32 = 1 << 6;
#[cfg(target_os = "macos")]
const EVENTFLAG_COMMAND_DOWN: u32 = 1 << 7;

pub fn handle_mouse_down(browser: &BrowserTab, event: &MouseDownEvent, offset: Point<Pixels>) {
    let position = event.position - offset;
    let x = f32::from(position.x) as i32;
    let y = f32::from(position.y) as i32;
    let button = convert_mouse_button(event.button);
    let click_count = event.click_count as i32;
    let modifiers = convert_modifiers(&event.modifiers);

    browser.send_mouse_click(x, y, button, true, click_count, modifiers);
}

pub fn handle_mouse_up(browser: &BrowserTab, event: &MouseUpEvent, offset: Point<Pixels>) {
    let position = event.position - offset;
    let x = f32::from(position.x) as i32;
    let y = f32::from(position.y) as i32;
    let button = convert_mouse_button(event.button);
    let modifiers = convert_modifiers(&event.modifiers);

    browser.send_mouse_click(x, y, button, false, 1, modifiers);
}

pub fn handle_mouse_move(browser: &BrowserTab, event: &MouseMoveEvent, offset: Point<Pixels>) {
    let position = event.position - offset;
    let x = f32::from(position.x) as i32;
    let y = f32::from(position.y) as i32;
    let mut modifiers = convert_modifiers(&event.modifiers);
    modifiers |= pressed_button_flags(event.pressed_button);

    browser.send_mouse_move(x, y, false, modifiers);
}

pub fn handle_scroll_wheel(browser: &BrowserTab, event: &ScrollWheelEvent, offset: Point<Pixels>) {
    let position = event.position - offset;
    let x = f32::from(position.x) as i32;
    let y = f32::from(position.y) as i32;

    let (delta_x, delta_y) = match event.delta {
        ScrollDelta::Pixels(delta) => (f32::from(delta.x) as i32, f32::from(delta.y) as i32),
        ScrollDelta::Lines(delta) => {
            let line_height = 40;
            (
                (delta.x * line_height as f32) as i32,
                (delta.y * line_height as f32) as i32,
            )
        }
    };

    let modifiers = convert_modifiers(&event.modifiers);

    browser.send_mouse_wheel(x, y, delta_x, delta_y, modifiers);
}

pub fn handle_key_down(browser: &BrowserTab, keystroke: &Keystroke, is_held: bool) {
    let raw_keydown = convert_key_event(keystroke, true);
    browser.send_key_event(&raw_keydown);

    if should_send_char_event(keystroke, is_held)
        && let Some(char_event) = create_char_event(keystroke)
    {
        browser.send_key_event(&char_event);
    }
}

pub fn handle_key_up(browser: &BrowserTab, keystroke: &Keystroke) {
    let keyup = convert_key_event(keystroke, false);
    browser.send_key_event(&keyup);
}

pub fn handle_text_commit(browser: &BrowserTab, text: &str) {
    match committed_text_action(text) {
        CommittedTextAction::PressEnter => send_synthetic_enter(browser),
        CommittedTextAction::InsertText => browser.insert_committed_text(text),
    }
}

fn pressed_button_flags(pressed_button: Option<MouseButton>) -> u32 {
    match pressed_button {
        Some(MouseButton::Left) | Some(MouseButton::Navigate(_)) => EVENTFLAG_LEFT_MOUSE_BUTTON,
        Some(MouseButton::Middle) => EVENTFLAG_MIDDLE_MOUSE_BUTTON,
        Some(MouseButton::Right) => EVENTFLAG_RIGHT_MOUSE_BUTTON,
        None => 0,
    }
}

fn convert_mouse_button(button: MouseButton) -> MouseButtonType {
    match button {
        MouseButton::Left | MouseButton::Navigate(_) => MouseButtonType::LEFT,
        MouseButton::Middle => MouseButtonType::MIDDLE,
        MouseButton::Right => MouseButtonType::RIGHT,
    }
}

fn convert_key_event(keystroke: &Keystroke, is_down: bool) -> KeyEvent {
    let modifiers = convert_modifiers(&keystroke.modifiers);
    let prefer_logical_key = keystroke.key_char.is_some()
        && !keystroke.modifiers.platform
        && !keystroke.modifiers.control
        && !keystroke.modifiers.alt
        && !keystroke.modifiers.function;
    let windows_key_code = if prefer_logical_key {
        key_name_to_windows_vk(&keystroke.key)
    } else {
        keystroke
            .native_key_code
            .map(macos_keycode_to_windows_vk)
            .unwrap_or_else(|| key_name_to_windows_vk(&keystroke.key))
    };
    let native_key_code = keystroke.native_key_code.unwrap_or(0) as i32;

    KeyEvent {
        type_: if is_down {
            KeyEventType::RAWKEYDOWN
        } else {
            KeyEventType::KEYUP
        },
        modifiers,
        windows_key_code,
        native_key_code,
        is_system_key: 0,
        character: key_character(keystroke),
        unmodified_character: unmodified_key_character(keystroke),
        focus_on_editable_field: 1,
        ..Default::default()
    }
}

fn create_char_event(keystroke: &Keystroke) -> Option<KeyEvent> {
    let character = key_character(keystroke);
    if character == 0 {
        return None;
    }

    Some(KeyEvent {
        type_: KeyEventType::CHAR,
        modifiers: convert_modifiers(&keystroke.modifiers),
        windows_key_code: character as i32,
        character,
        unmodified_character: character,
        focus_on_editable_field: 1,
        ..Default::default()
    })
}

fn should_send_char_event(keystroke: &Keystroke, is_held: bool) -> bool {
    if is_held || keystroke.modifiers.platform || keystroke.modifiers.control {
        return false;
    }

    !matches!(
        keystroke.key.as_str(),
        "backspace"
            | "tab"
            | "delete"
            | "escape"
            | "left"
            | "right"
            | "up"
            | "down"
            | "home"
            | "end"
            | "pageup"
            | "pagedown"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommittedTextAction {
    InsertText,
    PressEnter,
}

fn committed_text_action(text: &str) -> CommittedTextAction {
    if matches!(text, "\r" | "\n" | "\r\n") {
        CommittedTextAction::PressEnter
    } else {
        CommittedTextAction::InsertText
    }
}

fn send_synthetic_enter(browser: &BrowserTab) {
    for type_ in [
        KeyEventType::RAWKEYDOWN,
        KeyEventType::CHAR,
        KeyEventType::KEYUP,
    ] {
        browser.send_key_event(&KeyEvent {
            type_,
            modifiers: 0,
            windows_key_code: key_name_to_windows_vk("enter"),
            native_key_code: 0,
            is_system_key: 0,
            character: 0x0D,
            unmodified_character: 0x0D,
            focus_on_editable_field: 1,
            ..Default::default()
        });
    }
}

fn key_character(keystroke: &Keystroke) -> u16 {
    match keystroke.key.as_str() {
        "enter" => 0x0D,
        "backspace" => 0x08,
        "tab" => 0x09,
        "escape" => 0x1B,
        "space" => ' ' as u16,
        "delete" => 0x7F,
        _ => keystroke
            .key_char
            .as_ref()
            .and_then(|s| s.chars().next())
            .map(|c| c as u16)
            .or_else(|| {
                (keystroke.key.len() == 1)
                    .then(|| keystroke.key.chars().next())
                    .flatten()
                    .filter(|c| !c.is_control())
                    .map(|c| c as u16)
            })
            .unwrap_or(0),
    }
}

fn unmodified_key_character(keystroke: &Keystroke) -> u16 {
    match keystroke.key.as_str() {
        "enter" => 0x0D,
        "backspace" => 0x08,
        "tab" => 0x09,
        "escape" => 0x1B,
        "space" => ' ' as u16,
        "delete" => 0x7F,
        _ if keystroke.key.len() == 1 => {
            keystroke.key.chars().next().map(|c| c as u16).unwrap_or(0)
        }
        _ => 0,
    }
}

pub fn convert_modifiers(modifiers: &Modifiers) -> u32 {
    let mut result = 0u32;

    if modifiers.shift {
        result |= EVENTFLAG_SHIFT_DOWN;
    }
    if modifiers.control {
        result |= EVENTFLAG_CONTROL_DOWN;
    }
    if modifiers.alt {
        result |= EVENTFLAG_ALT_DOWN;
    }
    if modifiers.platform {
        #[cfg(target_os = "macos")]
        {
            result |= EVENTFLAG_COMMAND_DOWN;
        }
        #[cfg(not(target_os = "macos"))]
        {
            result |= EVENTFLAG_CONTROL_DOWN;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{CommittedTextAction, committed_text_action, should_send_char_event};
    use gpui::{Keystroke, Modifiers};

    fn keystroke(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            key: key.into(),
            key_char: key_char.map(str::to_string),
            modifiers,
            native_key_code: None,
        }
    }

    #[test]
    fn enter_emits_char_event() {
        let keystroke = keystroke("enter", None, Modifiers::default());

        assert!(should_send_char_event(&keystroke, false));
    }

    #[test]
    fn printable_text_emits_char_event() {
        let keystroke = keystroke("e", Some("e"), Modifiers::default());

        assert!(should_send_char_event(&keystroke, false));
    }

    #[test]
    fn committed_newline_maps_to_enter_keypress() {
        assert_eq!(committed_text_action("\n"), CommittedTextAction::PressEnter);
        assert_eq!(committed_text_action("\r"), CommittedTextAction::PressEnter);
        assert_eq!(
            committed_text_action("\r\n"),
            CommittedTextAction::PressEnter
        );
    }

    #[test]
    fn regular_committed_text_stays_text() {
        assert_eq!(
            committed_text_action("hello"),
            CommittedTextAction::InsertText
        );
        assert_eq!(
            committed_text_action("line 1\nline 2"),
            CommittedTextAction::InsertText
        );
    }
}
