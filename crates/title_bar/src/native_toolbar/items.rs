use gpui::{
    Action, App, NativeToolbarButton, NativeToolbarClickEvent, NativeToolbarControlGroup,
    NativeToolbarGroupControlRepresentation, NativeToolbarGroupEvent, NativeToolbarGroupOption,
    NativeToolbarItem, Window,
};
use workspace_chrome::{mode_index, mode_label, mode_sf_symbol};
use workspace_modes::{ModeId, SwitchToBrowserMode, SwitchToEditorMode, SwitchToTerminalMode};

use crate::TitleBar;

impl TitleBar {
    pub(crate) fn has_restricted_worktrees(&self, cx: &App) -> bool {
        project::trusted_worktrees::TrustedWorktrees::try_get_global(cx)
            .map(|trusted_worktrees| {
                trusted_worktrees
                    .read(cx)
                    .has_restricted_worktrees(&self.project.read(cx).worktree_store(), cx)
            })
            .unwrap_or(false)
    }

    pub(super) fn build_simple_action_button(
        &self,
        id: &'static str,
        icon: &'static str,
        tool_tip: &'static str,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> NativeToolbarItem {
        NativeToolbarItem::Button(
            NativeToolbarButton::new(id, "")
                .tool_tip(tool_tip)
                .icon(icon)
                .on_click(move |_: &NativeToolbarClickEvent, window, cx| on_click(window, cx)),
        )
    }

    pub(crate) fn build_mode_switcher_item(&self, active_mode: ModeId) -> NativeToolbarItem {
        let workspace = self.workspace.clone();
        NativeToolbarItem::ControlGroup(
            NativeToolbarControlGroup::new(
                "qedit.mode_switcher",
                vec![
                    NativeToolbarGroupOption::new(mode_label(ModeId::BROWSER))
                        .icon(mode_sf_symbol(ModeId::BROWSER))
                        .icon_only(),
                    NativeToolbarGroupOption::new(mode_label(ModeId::EDITOR))
                        .icon(mode_sf_symbol(ModeId::EDITOR))
                        .icon_only(),
                    NativeToolbarGroupOption::new(mode_label(ModeId::TERMINAL))
                        .icon(mode_sf_symbol(ModeId::TERMINAL))
                        .icon_only(),
                ],
            )
            .control_representation(NativeToolbarGroupControlRepresentation::Expanded)
            .selected_index(mode_index(active_mode))
            .on_select(move |event: &NativeToolbarGroupEvent, window, cx| {
                if workspace.upgrade().is_some() {
                    match event.selected_index {
                        0 => window.dispatch_action(SwitchToBrowserMode.boxed_clone(), cx),
                        1 => window.dispatch_action(SwitchToEditorMode.boxed_clone(), cx),
                        2 => window.dispatch_action(SwitchToTerminalMode.boxed_clone(), cx),
                        _ => {}
                    }
                }
            }),
        )
    }
}
