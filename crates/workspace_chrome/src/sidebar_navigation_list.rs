use gpui::{
    App, ClickEvent, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, Window, div, native_image_view, px,
};
use ui::{Color, Icon, IconButton, IconButtonShape, IconSize, Tooltip, prelude::*};

use crate::SidebarRow;

pub struct SidebarNavigationListItem {
    id: SharedString,
    label: SharedString,
    icon: IconName,
    image_uri: Option<SharedString>,
    selected: bool,
    pinned: bool,
    close_tooltip: SharedString,
    on_activate: Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    on_close: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_secondary_mouse_down: Option<Box<dyn Fn(Point<Pixels>, &mut Window, &mut App) + 'static>>,
}

impl SidebarNavigationListItem {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: IconName,
        on_activate: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            image_uri: None,
            selected: false,
            pinned: false,
            close_tooltip: SharedString::from("Close"),
            on_activate: Box::new(on_activate),
            on_close: None,
            on_secondary_mouse_down: None,
        }
    }

    pub fn image_uri(mut self, image_uri: Option<impl Into<SharedString>>) -> Self {
        self.image_uri = image_uri.map(Into::into);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn close_tooltip(mut self, close_tooltip: impl Into<SharedString>) -> Self {
        self.close_tooltip = close_tooltip.into();
        self
    }

    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Box::new(on_close));
        self
    }

    pub fn on_secondary_mouse_down(
        mut self,
        on_secondary_mouse_down: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_secondary_mouse_down = Some(Box::new(on_secondary_mouse_down));
        self
    }
}

#[derive(IntoElement)]
pub struct SidebarNavigationList {
    list_id: SharedString,
    empty_id: SharedString,
    empty_label: SharedString,
    empty_icon: IconName,
    items: Vec<SidebarNavigationListItem>,
}

impl SidebarNavigationList {
    pub fn new(
        list_id: impl Into<SharedString>,
        empty_id: impl Into<SharedString>,
        empty_label: impl Into<SharedString>,
        empty_icon: IconName,
        items: Vec<SidebarNavigationListItem>,
    ) -> Self {
        Self {
            list_id: list_id.into(),
            empty_id: empty_id.into(),
            empty_label: empty_label.into(),
            empty_icon,
            items,
        }
    }
}

impl RenderOnce for SidebarNavigationList {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let has_items = !self.items.is_empty();

        v_flex()
            .id(self.list_id)
            .flex_1()
            .items_stretch()
            .overflow_y_scroll()
            .p_1()
            .gap_1()
            .children(self.items.into_iter().map(|item| {
                let SidebarNavigationListItem {
                    id,
                    label,
                    icon,
                    image_uri,
                    selected,
                    pinned,
                    close_tooltip,
                    on_activate,
                    on_close,
                    on_secondary_mouse_down,
                } = item;

                let mut row = SidebarRow::new(id.clone(), label, icon).selected(selected);
                if let Some(image_uri) = image_uri {
                    row = row.start_slot(
                        native_image_view(SharedString::from(format!("{}-image", id.as_ref())))
                            .image_uri(image_uri.to_string())
                            .scaling(gpui::NativeImageScaling::ScaleUpOrDown)
                            .size(px(14.))
                            .rounded_sm()
                            .flex_shrink_0(),
                    );
                }

                let row = row
                    .end_slot(if pinned {
                        Icon::new(IconName::Pin)
                            .size(IconSize::Small)
                            .color(Color::Muted)
                            .into_any_element()
                    } else {
                        let close_id = id.clone();
                        IconButton::new(
                            SharedString::from(format!("{}-close", close_id.as_ref())),
                            IconName::Close,
                        )
                        .shape(IconButtonShape::Square)
                        .icon_size(IconSize::XSmall)
                        .icon_color(Color::Muted)
                        .tooltip(Tooltip::text(close_tooltip))
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            if let Some(on_close) = on_close.as_ref() {
                                on_close(event, window, cx);
                            }
                        })
                        .into_any_element()
                    })
                    .on_click(move |event, window, cx| {
                        on_activate(event, window, cx);
                    });

                div().w_full().child(row).when_some(
                    on_secondary_mouse_down,
                    |this, on_secondary_mouse_down| {
                        this.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            on_secondary_mouse_down(event.position, window, cx);
                        })
                    },
                )
            }))
            .when(!has_items, |this| {
                this.child(
                    SidebarRow::new(self.empty_id, self.empty_label, self.empty_icon)
                        .disabled(true),
                )
            })
    }
}
