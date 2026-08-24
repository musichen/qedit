use crate::TitleBar;
use gpui::{Action, App, NativeToolbarButton, NativeToolbarItem};
use workspace::ToggleWorktreeSecurity;

impl TitleBar {
    pub(crate) fn build_restricted_mode_item(&self, cx: &App) -> Option<NativeToolbarItem> {
        self.has_restricted_worktrees(cx).then(|| {
            NativeToolbarItem::Button(
                NativeToolbarButton::new("qedit.restricted_mode", "Restricted Mode")
                    .tool_tip("Manage Worktree Trust")
                    .icon("exclamationmark.shield")
                    .on_click(|_, window, cx| {
                        window.dispatch_action(ToggleWorktreeSecurity.boxed_clone(), cx);
                    }),
            )
        })
    }
}
