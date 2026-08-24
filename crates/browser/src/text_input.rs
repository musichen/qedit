use cef::{
    CefString, Domnode, Frame, ImplDomnode, ImplFrame, ImplListValue, ImplProcessMessage,
    ProcessId, ProcessMessage, process_message_create,
};
use gpui::Keystroke;

pub(crate) const TEXT_INPUT_STATE_MESSAGE_NAME: &str = "qedit.text_input_state";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BrowserTextInputState {
    pub(crate) editable: bool,
}

impl BrowserTextInputState {
    pub(crate) fn is_active(self, has_marked_text: bool) -> bool {
        self.editable || has_marked_text
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrowserKeyDispatch {
    App,
    Browser,
    TextInput,
}

pub(crate) fn extract_text_input_state_from_message(
    message: &mut ProcessMessage,
) -> Option<BrowserTextInputState> {
    if CefString::from(&message.name()).to_string() != TEXT_INPUT_STATE_MESSAGE_NAME {
        return None;
    }

    let args = message.argument_list()?;
    Some(BrowserTextInputState {
        editable: args.bool(0) != 0,
    })
}

pub(crate) fn send_text_input_state(frame: &mut Frame, focused_node: Option<&Domnode>) -> bool {
    let Some(message) =
        process_message_create(Some(&CefString::from(TEXT_INPUT_STATE_MESSAGE_NAME)))
    else {
        return false;
    };

    let Some(args) = message.argument_list() else {
        return false;
    };

    args.set_bool(
        0,
        focused_node.is_some_and(|node| node.is_editable() != 0) as i32,
    );

    let mut message = message;
    frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
    true
}

pub(crate) fn key_down_dispatch(
    keystroke: &Keystroke,
    text_input_editable: bool,
    text_input_composing: bool,
) -> BrowserKeyDispatch {
    if keystroke.modifiers.platform || keystroke.modifiers.control {
        BrowserKeyDispatch::App
    } else if should_use_text_input(keystroke, text_input_editable, text_input_composing) {
        BrowserKeyDispatch::TextInput
    } else {
        BrowserKeyDispatch::Browser
    }
}

pub(crate) fn key_up_dispatch(
    keystroke: &Keystroke,
    text_input_editable: bool,
    text_input_composing: bool,
) -> BrowserKeyDispatch {
    if keystroke.modifiers.platform || keystroke.modifiers.control {
        BrowserKeyDispatch::App
    } else if should_use_text_input(keystroke, text_input_editable, text_input_composing) {
        BrowserKeyDispatch::TextInput
    } else {
        BrowserKeyDispatch::Browser
    }
}

fn should_use_text_input(
    keystroke: &Keystroke,
    text_input_editable: bool,
    text_input_composing: bool,
) -> bool {
    if text_input_composing {
        return true;
    }

    if !text_input_editable {
        return false;
    }

    if matches!(keystroke.key.as_str(), "enter" | "tab") {
        return false;
    }

    if keystroke.key_char.is_some() {
        return true;
    }

    !matches!(
        keystroke.key.as_str(),
        "enter"
            | "backspace"
            | "tab"
            | "delete"
            | "escape"
            | "space"
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

#[cfg(test)]
mod tests {
    use super::{BrowserKeyDispatch, key_down_dispatch, key_up_dispatch};
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
    fn printable_keys_use_browser_route_when_page_is_not_editable() {
        let keystroke = keystroke("e", Some("e"), Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, false, false),
            BrowserKeyDispatch::Browser
        );
        assert_eq!(
            key_up_dispatch(&keystroke, false, false),
            BrowserKeyDispatch::Browser
        );
    }

    #[test]
    fn printable_keys_use_text_input_when_page_is_editable() {
        let keystroke = keystroke("e", Some("e"), Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
    }

    #[test]
    fn navigation_keys_still_reach_browser_when_page_is_editable() {
        let keystroke = keystroke("left", None, Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::Browser
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::Browser
        );
    }

    #[test]
    fn enter_routes_to_browser_even_when_platform_reports_character_input() {
        let keystroke = keystroke("enter", Some("\r"), Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::Browser
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::Browser
        );
    }

    #[test]
    fn command_shortcuts_stay_in_app_dispatch() {
        let keystroke = keystroke("c", Some("c"), Modifiers::command());

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::App
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::App
        );
    }

    #[test]
    fn function_printable_keys_use_browser_route_when_page_is_not_editable() {
        let keystroke = keystroke(
            "e",
            Some("e"),
            Modifiers {
                function: true,
                ..Modifiers::default()
            },
        );

        assert_eq!(
            key_down_dispatch(&keystroke, false, false),
            BrowserKeyDispatch::Browser
        );
        assert_eq!(
            key_up_dispatch(&keystroke, false, false),
            BrowserKeyDispatch::Browser
        );
    }

    #[test]
    fn function_printable_keys_use_text_input_when_page_is_editable() {
        let keystroke = keystroke(
            "e",
            Some("e"),
            Modifiers {
                function: true,
                ..Modifiers::default()
            },
        );

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
    }

    #[test]
    fn composing_keys_stay_in_text_input_route() {
        let keystroke = keystroke("e", Some("e"), Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, true, true),
            BrowserKeyDispatch::TextInput
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, true),
            BrowserKeyDispatch::TextInput
        );
    }

    #[test]
    fn dead_keys_stay_in_text_input_route_for_editable_fields() {
        let keystroke = keystroke("dead-acute", None, Modifiers::default());

        assert_eq!(
            key_down_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
        assert_eq!(
            key_up_dispatch(&keystroke, true, false),
            BrowserKeyDispatch::TextInput
        );
    }
}
