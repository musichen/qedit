#[cfg(target_os = "macos")]
mod app_store_connect_auth;
mod app_store_connect_provider;
mod command_runner;
#[cfg(target_os = "macos")]
mod service_auth;
mod service_hub_onboarding;
mod service_workflow;
mod services_page;
mod services_provider;

use gpui::{App, actions};
use services_page::ServicesPage;
use workspace::Workspace;

actions!(
    service_hub,
    [
        /// Opens service management for the current workspace.
        OpenServices,
        /// Opens the Service Hub onboarding surface.
        ShowOnboarding,
        /// Resets the Service Hub onboarding state.
        ResetOnboarding
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(
        |workspace: &mut Workspace,
         window: Option<&mut gpui::Window>,
         _cx: &mut gpui::Context<Workspace>| {
            let Some(_) = window else {
                return;
            };

            workspace
                .register_action(move |workspace, _: &OpenServices, window, cx| {
                    ServicesPage::open(workspace, false, window, cx);
                })
                .register_action(move |workspace, _: &ShowOnboarding, window, cx| {
                    ServicesPage::open(workspace, true, window, cx);
                })
                .register_action(move |workspace, _: &ResetOnboarding, window, cx| {
                    ServicesPage::reset_onboarding(workspace, window, cx);
                });
        },
    )
    .detach();
}
