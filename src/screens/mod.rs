pub mod auth;
pub mod home;

use gpui::{div, prelude::*};
use gpui_mobile::{set_system_chrome, StatusBarContentStyle, SystemChromeStyle};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::api::{GitHubUser, Notification};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Auth,
    Home,
}

pub enum AuthPhase {
    Idle,
    RequestingCode,
    WaitingForUser {
        user_code: String,
        verification_uri: String,
        device_code: String,
        interval: u64,
    },
    Polling,
    Error(String),
}

pub struct Router {
    pub current_screen: Screen,
    pub auth_phase: AuthPhase,
    pub polling_cancel: Option<Arc<AtomicBool>>,
    pub token: Option<String>,
    pub user: Option<GitHubUser>,
    pub notifications: Vec<Notification>,
    pub notifications_loading: bool,
    pub search_query: String,
}

impl Router {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Auth,
            auth_phase: AuthPhase::Idle,
            polling_cancel: None,
            token: None,
            user: None,
            notifications: Vec::new(),
            notifications_loading: false,
            search_query: String::new(),
        }
    }

    pub fn navigate_to(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    pub fn start_device_flow(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(cancel) = self.polling_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        self.auth_phase = AuthPhase::RequestingCode;
        cx.notify();

        let cancel = Arc::new(AtomicBool::new(false));
        self.polling_cancel = Some(cancel.clone());

        cx.spawn(|entity, mut async_cx| async move {
            let client = match crate::api::make_client() {
                Ok(c) => c,
                Err(e) => {
                    entity.update(&mut async_cx, |router, cx| {
                        router.auth_phase = AuthPhase::Error(format!("Client error: {e}"));
                        cx.notify();
                    }).ok();
                    return;
                }
            };

            let code_resp = match crate::api::request_device_code(&client).await {
                Ok(r) => r,
                Err(e) => {
                    entity.update(&mut async_cx, |router, cx| {
                        router.auth_phase = AuthPhase::Error(format!("Failed to start auth: {e}"));
                        cx.notify();
                    }).ok();
                    return;
                }
            };

            let device_code = code_resp.device_code.clone();
            let interval = code_resp.interval.max(5);

            entity.update(&mut async_cx, |router, cx| {
                router.auth_phase = AuthPhase::WaitingForUser {
                    user_code: code_resp.user_code.clone(),
                    verification_uri: code_resp.verification_uri.clone(),
                    device_code: code_resp.device_code.clone(),
                    interval,
                };
                cx.notify();
            }).ok();

            loop {
                if cancel.load(Ordering::Acquire) {
                    break;
                }

                smol::Timer::after(std::time::Duration::from_secs(interval)).await;

                if cancel.load(Ordering::Acquire) {
                    break;
                }

                let poll_result = crate::api::poll_token(&client, &device_code).await;

                let should_break = entity.update(&mut async_cx, |router, cx| {
                    match poll_result {
                        Ok(crate::api::TokenResponse { access_token: Some(new_token), .. }) => {
                            let t = new_token.clone();
                            router.token = Some(new_token);
                            router.polling_cancel = None;
                            router.navigate_to(Screen::Home);
                            cx.notify();
                            router.load_home_data(cx, t);
                            true
                        }
                        Ok(crate::api::TokenResponse { error: Some(ref e), .. })
                            if e == "authorization_pending" || e == "slow_down" =>
                        {
                            false
                        }
                        Ok(crate::api::TokenResponse { error: Some(e), .. }) => {
                            router.auth_phase = AuthPhase::Error(format!("Auth denied: {e}"));
                            cx.notify();
                            true
                        }
                        Ok(_) => false,
                        Err(e) => {
                            router.auth_phase = AuthPhase::Error(format!("Network error: {e}"));
                            cx.notify();
                            true
                        }
                    }
                }).unwrap_or(true);

                if should_break {
                    break;
                }
            }
        }).detach();
    }

    pub fn load_home_data(&mut self, cx: &mut gpui::Context<Self>, token: String) {
        self.notifications_loading = true;
        cx.notify();

        cx.spawn(|entity, mut async_cx| async move {
            let client = match crate::api::make_client() {
                Ok(c) => c,
                Err(_) => return,
            };

            let user_result = crate::api::get_user(&client, &token).await;
            let notifs_result = crate::api::get_notifications(&client, &token).await;

            entity.update(&mut async_cx, |router, cx| {
                if let Ok(user) = user_result {
                    router.user = Some(user);
                }
                router.notifications = notifs_result.unwrap_or_default();
                router.notifications_loading = false;
                cx.notify();
            }).ok();
        }).detach();
    }

    pub fn refresh_notifications(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(token) = self.token.clone() {
            self.load_home_data(cx, token);
        }
    }
}

impl gpui::Render for Router {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        set_system_chrome(SystemChromeStyle {
            status_bar: StatusBarContentStyle::Light,
            navigation_bar: StatusBarContentStyle::Light,
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(gpui::rgb(0x0D1117))
            .child(match self.current_screen {
                Screen::Auth => auth::render(self, cx).into_any_element(),
                Screen::Home => home::render(self, cx).into_any_element(),
            })
    }
}
