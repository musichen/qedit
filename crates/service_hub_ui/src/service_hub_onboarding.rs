use gpui::{AnyElement, App, RenderOnce, SharedString, WeakEntity, Window, px};
use service_hub::{ServiceAuthKind, ServiceProviderDescriptor};
use ui::{
    Button, ButtonSize, ButtonStyle, Color, Divider, Icon, IconButton, IconButtonShape, IconName,
    IconSize, Label, LabelSize, h_flex, prelude::*, rems_from_px, v_flex,
};
use workspace_chrome::SidebarRow;

use crate::services_page::ServicesPage;

const ASC_CLI_INSTALL_URL: &str = "https://github.com/rudrankriyam/App-Store-Connect-CLI#1-install";
const ASC_CLI_GITHUB_URL: &str = "https://github.com/rudrankriyam/App-Store-Connect-CLI";

#[derive(IntoElement)]
pub(crate) struct ServiceHubOnboarding {
    page: WeakEntity<ServicesPage>,
    providers: Vec<ServiceProviderDescriptor>,
}

struct ProviderPresentation {
    description: String,
    highlights: Vec<String>,
    open_label: String,
}

#[derive(IntoElement)]
struct SectionHeader {
    title: SharedString,
}

impl SectionHeader {
    fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .px_1()
            .mb_2()
            .gap_2()
            .child(
                Label::new(self.title.to_ascii_uppercase())
                    .buffer_font(cx)
                    .color(Color::Muted)
                    .size(LabelSize::XSmall),
            )
            .child(Divider::horizontal().color(ui::DividerColor::BorderVariant))
    }
}

impl ServiceHubOnboarding {
    pub(crate) fn new(
        page: WeakEntity<ServicesPage>,
        providers: Vec<ServiceProviderDescriptor>,
    ) -> Self {
        Self { page, providers }
    }

    fn presentation(provider: &ServiceProviderDescriptor) -> ProviderPresentation {
        match provider.id.as_str() {
            "app-store-connect" => ProviderPresentation {
                description:
                    "Manage App Store work from one place inside Qedit. Requires the local ASC CLI."
                        .to_string(),
                highlights: vec![
                    "Install ASC CLI once on this machine".to_string(),
                    "Browse your apps and recent builds".to_string(),
                    "Check TestFlight and App Store status".to_string(),
                    "Publish when a build is ready".to_string(),
                ],
                open_label: "Open App Store Connect".to_string(),
            },
            _ => {
                let resource_highlight = provider
                    .shell
                    .resource_kind
                    .as_ref()
                    .map(|resource_kind| format!("Browse {}", resource_kind.plural_label))
                    .unwrap_or_else(|| "Open your connected work".to_string());
                let auth_highlight = match provider.auth_kind {
                    ServiceAuthKind::None => "Jump in right away".to_string(),
                    ServiceAuthKind::ApiKey | ServiceAuthKind::OAuth => {
                        "Connect your account when you are ready".to_string()
                    }
                };
                let action_highlight = if provider.workflows.is_empty() {
                    "Keep the important status in one place".to_string()
                } else {
                    "Run the next step without leaving Qedit".to_string()
                };

                ProviderPresentation {
                    description: format!("Bring your {} work into Qedit.", provider.label),
                    highlights: vec![resource_highlight, action_highlight, auth_highlight],
                    open_label: format!("Open {}", provider.label),
                }
            }
        }
    }

    fn provider_logo(provider: &ServiceProviderDescriptor, size: f32) -> AnyElement {
        provider
            .logo_asset_path
            .as_deref()
            .map(|path| {
                gpui::img(path)
                    .w(rems_from_px(size))
                    .h(rems_from_px(size))
                    .rounded(px(4.0))
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                Icon::new(IconName::Server)
                    .size(IconSize::Small)
                    .color(Color::Muted)
                    .into_any_element()
            })
    }

    fn render_highlight_row(text: impl Into<SharedString>, cx: &mut App) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_2()
            .child(
                Icon::new(IconName::Check)
                    .size(IconSize::Small)
                    .color(Color::Accent),
            )
            .child(
                Label::new(text)
                    .size(LabelSize::Small)
                    .color(cx.theme().colors().text.into()),
            )
    }

    fn render_provider_card(
        &self,
        provider: &ServiceProviderDescriptor,
        cx: &mut App,
    ) -> impl IntoElement + use<> {
        let radius = cx.theme().component_radius().panel.unwrap_or(px(10.0));
        let presentation = Self::presentation(provider);
        let provider_id = provider.id.clone();
        let page = self.page.clone();
        let highlights = presentation
            .highlights
            .iter()
            .map(|highlight| Self::render_highlight_row(highlight.clone(), cx).into_any_element())
            .collect::<Vec<_>>();
        let logo_tile = h_flex()
            .size(rems_from_px(36.0))
            .items_center()
            .justify_center()
            .rounded(px(8.0))
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().editor_background)
            .child(Self::provider_logo(provider, 22.0));

        v_flex()
            .gap_4()
            .p_5()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .bg(cx.theme().colors().background)
            .child(
                h_flex().items_start().gap_3().child(logo_tile).child(
                    v_flex()
                        .gap_1()
                        .child(Label::new(provider.label.clone()).size(LabelSize::Large))
                        .child(
                            Label::new(presentation.description)
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                ),
            )
            .child(v_flex().gap_2().children(highlights))
            .when(provider.id == "app-store-connect", |this| {
                this.child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .gap_3()
                        .flex_wrap()
                        .child(
                            Label::new("Requires local ASC CLI")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    Button::new("service-hub-onboarding-asc-install", "Install")
                                        .style(ButtonStyle::Subtle)
                                        .size(ButtonSize::Compact)
                                        .on_click(|_, _, cx| {
                                            cx.open_url(ASC_CLI_INSTALL_URL);
                                        }),
                                )
                                .child(
                                    IconButton::new(
                                        "service-hub-onboarding-asc-cli",
                                        IconName::Github,
                                    )
                                    .shape(IconButtonShape::Square)
                                    .style(ButtonStyle::Transparent)
                                    .size(ButtonSize::Compact)
                                    .icon_size(IconSize::Small)
                                    .tooltip(ui::Tooltip::text("View ASC CLI on GitHub"))
                                    .on_click(|_, _, cx| {
                                        cx.open_url(ASC_CLI_GITHUB_URL);
                                    }),
                                ),
                        ),
                )
            })
            .child(
                SidebarRow::new(
                    format!("service-hub-onboarding-open-{}", provider.id),
                    presentation.open_label,
                    IconName::ChevronRight,
                )
                .start_slot(Self::provider_logo(provider, 16.0))
                .on_click(move |_, window, cx| {
                    page.update(cx, |page, cx| {
                        page.complete_onboarding(Some(provider_id.clone()), window, cx);
                    })
                    .ok();
                }),
            )
    }
}

impl RenderOnce for ServiceHubOnboarding {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let section_title = if self.providers.len() > 1 {
            "Providers"
        } else {
            "Start Here"
        };
        let provider_cards = self
            .providers
            .iter()
            .map(|provider| self.render_provider_card(provider, cx).into_any_element())
            .collect::<Vec<_>>();

        div()
            .image_cache(gpui::retain_all("service-hub-onboarding"))
            .size_full()
            .child(
            div()
                .max_w(Rems(48.0))
                .size_full()
                .mx_auto()
                .child(
                    v_flex()
                        .id("service-hub-onboarding")
                        .m_auto()
                        .p_12()
                        .size_full()
                        .max_w_full()
                        .min_w_0()
                        .gap_6()
                        .child(
                            h_flex()
                                .w_full()
                                .gap_4()
                                .items_start()
                                .child({
                                    let logo = match cx.theme().appearance {
                                        theme::Appearance::Light => "images/qedit_logo_light.png",
                                        theme::Appearance::Dark => "images/qedit_logo_dark.png",
                                    };
                                    gpui::img(logo).w(rems(2.5)).h(rems(2.5))
                                })
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .child(
                                            Headline::new("Welcome to Service Hub")
                                                .size(HeadlineSize::Small),
                                        )
                                        .child(
                                            Label::new(
                                                "Keep release work close to your code. Check status, review builds, and publish without bouncing between tools.",
                                            )
                                            .color(Color::Muted)
                                            .size(LabelSize::Small),
                                        ),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_6()
                                .child(SectionHeader::new(section_title))
                                .children(provider_cards),
                        ),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use service_hub::{
        ServiceAuthKind, ServiceNavigationItemDescriptor, ServiceProviderDescriptor,
        ServiceShellDescriptor,
    };

    use super::ServiceHubOnboarding;

    fn test_provider(
        id: &str,
        auth_kind: ServiceAuthKind,
        logo_asset_path: Option<&str>,
    ) -> ServiceProviderDescriptor {
        ServiceProviderDescriptor {
            id: id.to_string(),
            label: id.to_string(),
            logo_asset_path: logo_asset_path.map(ToString::to_string),
            shell: ServiceShellDescriptor {
                resource_kind: None,
                navigation_items: vec![ServiceNavigationItemDescriptor {
                    id: "overview".to_string(),
                    label: "Overview".to_string(),
                }],
                default_navigation_item_id: "overview".to_string(),
            },
            auth_kind,
            auth: None,
            targets: Vec::new(),
            workflows: Vec::new(),
        }
    }

    #[test]
    fn keeps_provider_logo_metadata_on_cards() {
        let onboarding = ServiceHubOnboarding::new(
            gpui::WeakEntity::new_invalid(),
            vec![
                test_provider(
                    "app-store-connect",
                    ServiceAuthKind::ApiKey,
                    Some("images/asc_logo.png"),
                ),
                test_provider("vercel", ServiceAuthKind::OAuth, None),
            ],
        );

        assert_eq!(
            onboarding.providers[0].logo_asset_path.as_deref(),
            Some("images/asc_logo.png")
        );
        assert_eq!(onboarding.providers[1].logo_asset_path, None);
    }

    #[test]
    fn app_store_connect_presentation_mentions_cli_requirement() {
        let presentation = ServiceHubOnboarding::presentation(&test_provider(
            "app-store-connect",
            ServiceAuthKind::ApiKey,
            None,
        ));

        assert!(presentation.description.contains("ASC CLI"));
        assert!(
            presentation
                .highlights
                .iter()
                .any(|highlight| highlight.contains("Install ASC CLI"))
        );
    }
}
