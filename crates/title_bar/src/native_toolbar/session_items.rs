use client::Status as ClientStatus;
use gpui::{App, NativeToolbarButton, NativeToolbarItem};

use crate::TitleBar;

impl TitleBar {
    pub(crate) fn build_connection_status_item(&self, _cx: &App) -> Option<NativeToolbarItem> {
        match &*self.client.status().borrow() {
            ClientStatus::ConnectionError
            | ClientStatus::ConnectionLost
            | ClientStatus::Reauthenticating
            | ClientStatus::Reconnecting
            | ClientStatus::ReconnectionError { .. } => Some(NativeToolbarItem::Button(
                NativeToolbarButton::new("qedit.connection_status", "Disconnected")
                    .tool_tip("Disconnected")
                    .icon("wifi.exclamationmark"),
            )),
            ClientStatus::UpgradeRequired => Some(NativeToolbarItem::Button(
                NativeToolbarButton::new("qedit.connection_status", "Update Required")
                    .tool_tip("Please Update to Collaborate")
                    .icon("exclamationmark.arrow.circlepath")
                    .on_click(|_, window, cx| {
                        auto_update::check(&Default::default(), window, cx);
                    }),
            )),
            _ => None,
        }
    }

    pub(crate) fn build_update_item(&self) -> NativeToolbarItem {
        self.build_simple_action_button(
            "qedit.update",
            "arrow.down.circle",
            "Restart to Update",
            |_window, cx| {
                workspace::reload(cx);
            },
        )
    }
}
