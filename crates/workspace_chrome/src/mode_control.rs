use gpui::{App, RenderOnce, Window};
use std::sync::Arc;
use ui::{
    ToggleButtonGroup, ToggleButtonGroupSize, ToggleButtonGroupStyle, ToggleButtonWithIcon,
    Tooltip, prelude::*,
};
use workspace_modes::ModeId;

pub type OnModeSelect = Arc<dyn Fn(ModeId, &mut Window, &mut App) + Send + Sync>;

pub fn mode_label(mode_id: ModeId) -> &'static str {
    match mode_id {
        ModeId::BROWSER => "Browser",
        ModeId::EDITOR => "Editor",
        ModeId::TERMINAL => "Terminal",
        _ => "Browser",
    }
}

pub fn mode_icon(mode_id: ModeId) -> IconName {
    match mode_id {
        ModeId::BROWSER => IconName::Globe,
        ModeId::EDITOR => IconName::FileCode,
        ModeId::TERMINAL => IconName::Terminal,
        _ => IconName::Globe,
    }
}

pub fn mode_sf_symbol(mode_id: ModeId) -> &'static str {
    match mode_id {
        ModeId::BROWSER => "globe",
        ModeId::EDITOR => "doc.text",
        ModeId::TERMINAL => "terminal",
        _ => "globe",
    }
}

pub fn mode_index(mode_id: ModeId) -> usize {
    match mode_id {
        ModeId::BROWSER => 0,
        ModeId::EDITOR => 1,
        ModeId::TERMINAL => 2,
        _ => 0,
    }
}

fn mode_from_index(index: usize) -> Option<ModeId> {
    match index {
        0 => Some(ModeId::BROWSER),
        1 => Some(ModeId::EDITOR),
        2 => Some(ModeId::TERMINAL),
        _ => None,
    }
}

#[derive(IntoElement)]
pub struct ModeControl {
    active_mode_id: ModeId,
    on_mode_select: Option<OnModeSelect>,
}

impl ModeControl {
    pub fn new(active_mode_id: ModeId) -> Self {
        Self {
            active_mode_id,
            on_mode_select: None,
        }
    }

    pub fn on_mode_select(
        mut self,
        callback: impl Fn(ModeId, &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.on_mode_select = Some(Arc::new(callback));
        self
    }
}

impl RenderOnce for ModeControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_mode_select = self.on_mode_select;
        let buttons = [0, 1, 2].map(|index| {
            let mode_id = mode_from_index(index).expect("valid mode index");
            let callback = on_mode_select.clone();
            ToggleButtonWithIcon::new("", mode_icon(mode_id), move |_, window, cx| {
                if let Some(callback) = callback.as_ref() {
                    callback(mode_id, window, cx);
                }
            })
            .tooltip(Tooltip::text(mode_label(mode_id)))
        });

        ToggleButtonGroup::single_row("workspace-mode-control", buttons)
            .style(ToggleButtonGroupStyle::Outlined)
            .size(ToggleButtonGroupSize::Medium)
            .selected_index(mode_index(self.active_mode_id))
            .auto_width()
    }
}
