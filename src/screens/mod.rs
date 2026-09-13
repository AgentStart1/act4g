pub mod auth;
pub mod detail;
pub mod home;

use gpui::{div, prelude::*};
use gpui_mobile::{set_system_chrome, StatusBarContentStyle, SystemChromeStyle};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::api::{GitHubUser, Notification, NotificationDetail};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Auth,
    Home,
    Detail,
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
    pub home_error: Option<String>,
    pub search_query: String,
    pub selected_notification: Option<Notification>,
    pub notification_detail: Option<NotificationDetail>,
    pub notification_detail_loading: bool,
    pub notification_detail_error: Option<String>,
    pub auth_request_id: u64,
    pub home_request_id: u64,
    pub detail_request_id: u64,
}

impl Router {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        let saved_token = crate::credentials::load_token();
        let mut router = Self {
            current_screen: if saved_token.is_some() {
                Screen::Home
            } else {
                Screen::Auth
            },
            auth_phase: AuthPhase::Idle,
            polling_cancel: None,
            token: saved_token.clone(),
            user: None,
            notifications: Vec::new(),
            notifications_loading: false,
            home_error: None,
            search_query: String::new(),
            selected_notification: None,
            notification_detail: None,
            notification_detail_loading: false,
            notification_detail_error: None,
            auth_request_id: 0,
            home_request_id: 0,
            detail_request_id: 0,
        };

        if let Some(token) = saved_token {
            router.load_home_data(cx, token);
        }

        router
    }

    pub fn navigate_to(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    pub fn start_device_flow(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(cancel) = self.polling_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        self.auth_request_id = self.auth_request_id.wrapping_add(1);
        let request_id = self.auth_request_id;
        self.auth_phase = AuthPhase::RequestingCode;
        cx.notify();

        let cancel = Arc::new(AtomicBool::new(false));
        self.polling_cancel = Some(cancel.clone());

        cx.spawn(async move |entity: gpui::WeakEntity<Self>, cx| {
            let client = match crate::api::make_client() {
                Ok(c) => c,
                Err(e) => {
                    entity
                        .update(cx, |router, cx| {
                            if router.auth_request_id != request_id
                                || cancel.load(Ordering::Acquire)
                            {
                                return;
                            }
                            router.auth_phase = AuthPhase::Error(format!("Client error: {e}"));
                            router.polling_cancel = None;
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let code_resp = match crate::api::request_device_code(&client).await {
                Ok(r) => r,
                Err(e) => {
                    entity
                        .update(cx, |router, cx| {
                            if router.auth_request_id != request_id
                                || cancel.load(Ordering::Acquire)
                            {
                                return;
                            }
                            router.auth_phase =
                                AuthPhase::Error(format!("Failed to start auth: {e}"));
                            router.polling_cancel = None;
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let device_code = code_resp.device_code.clone();
            let interval = code_resp.interval.max(5);
            let expires_at =
                std::time::Instant::now() + std::time::Duration::from_secs(code_resp.expires_in);

            let flow_active = entity
                .update(cx, |router, cx| {
                    if router.auth_request_id != request_id || cancel.load(Ordering::Acquire) {
                        return false;
                    }
                    router.auth_phase = AuthPhase::WaitingForUser {
                        user_code: code_resp.user_code.clone(),
                        verification_uri: code_resp.verification_uri.clone(),
                        device_code: code_resp.device_code.clone(),
                        interval,
                    };
                    cx.notify();
                    true
                })
                .unwrap_or(false);

            if !flow_active {
                return;
            }

            let authorization_url =
                device_authorization_url(&code_resp.verification_uri, &code_resp.user_code);
            match gpui_mobile::packages::url_launcher::launch_url(&authorization_url) {
                Ok(true) => {}
                Ok(false) => log::warn!("No app could open the GitHub authorization URL"),
                Err(error) => log::warn!("Failed to open GitHub authorization URL: {error}"),
            }

            let mut poll_interval = interval;

            loop {
                if cancel.load(Ordering::Acquire) {
                    break;
                }

                if std::time::Instant::now() >= expires_at {
                    entity
                        .update(cx, |router, cx| {
                            if router.auth_request_id != request_id
                                || cancel.load(Ordering::Acquire)
                            {
                                return;
                            }
                            router.auth_phase = AuthPhase::Error(
                                "The device code expired. Start sign in again to get a new code."
                                    .to_string(),
                            );
                            router.polling_cancel = None;
                            cx.notify();
                        })
                        .ok();
                    break;
                }

                smol::Timer::after(std::time::Duration::from_secs(poll_interval)).await;

                if cancel.load(Ordering::Acquire) {
                    break;
                }

                let poll_result = crate::api::poll_token(&client, &device_code).await;

                if cancel.load(Ordering::Acquire) {
                    break;
                }

                if let Err(error) = &poll_result {
                    log::warn!(
                        "Token poll failed; retrying while the device code is valid: {error:#}"
                    );
                    continue;
                }

                if matches!(
                    &poll_result,
                    Ok(crate::api::TokenResponse { error: Some(error), .. }) if error == "slow_down"
                ) {
                    poll_interval += 5;
                }

                let should_break = entity
                    .update(cx, |router, cx| {
                        if router.auth_request_id != request_id || cancel.load(Ordering::Acquire) {
                            return true;
                        }

                        match poll_result {
                            Ok(crate::api::TokenResponse {
                                access_token: Some(new_token),
                                ..
                            }) => {
                                let t = new_token.clone();
                                if let Err(error) = crate::credentials::save_token(&new_token) {
                                    log::warn!("Failed to persist GitHub session: {error}");
                                }
                                router.token = Some(new_token);
                                router.polling_cancel = None;
                                router.navigate_to(Screen::Home);
                                cx.notify();
                                router.load_home_data(cx, t);
                                true
                            }
                            Ok(crate::api::TokenResponse {
                                error: Some(ref e), ..
                            }) if e == "authorization_pending" || e == "slow_down" => false,
                            Ok(crate::api::TokenResponse { error: Some(e), .. }) => {
                                router.auth_phase = AuthPhase::Error(format!("Auth denied: {e}"));
                                cx.notify();
                                true
                            }
                            Ok(_) => false,
                            Err(_) => false,
                        }
                    })
                    .unwrap_or(true);

                if should_break {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn cancel_device_flow(&mut self, cx: &mut gpui::Context<Self>) {
        self.auth_request_id = self.auth_request_id.wrapping_add(1);
        if let Some(cancel) = self.polling_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        self.auth_phase = AuthPhase::Idle;
        cx.notify();
    }

    pub fn load_home_data(&mut self, cx: &mut gpui::Context<Self>, token: String) {
        self.home_request_id = self.home_request_id.wrapping_add(1);
        let request_id = self.home_request_id;
        self.notifications_loading = true;
        self.home_error = None;
        cx.notify();

        cx.spawn(async move |entity: gpui::WeakEntity<Self>, cx| {
            let client = match crate::api::make_client() {
                Ok(c) => c,
                Err(error) => {
                    log::error!("Failed to create GitHub client: {error:#}");
                    entity
                        .update(cx, |router, cx| {
                            router.notifications_loading = false;
                            router.home_error = Some(
                                "GitHub could not be reached. Check your connection and try again."
                                    .to_string(),
                            );
                            cx.notify();
                        })
                        .ok();
                    return;
                }
            };

            let user_result = crate::api::get_user(&client, &token).await;
            let notifs_result = crate::api::get_notifications(&client, &token).await;
            let session_expired = user_result
                .as_ref()
                .err()
                .is_some_and(crate::api::is_unauthorized)
                || notifs_result
                    .as_ref()
                    .err()
                    .is_some_and(crate::api::is_unauthorized);

            entity
                .update(cx, |router, cx| {
                    if router.home_request_id != request_id
                        || router.token.as_deref() != Some(token.as_str())
                    {
                        return;
                    }

                    if session_expired {
                        if let Err(error) = crate::credentials::clear_token() {
                            log::warn!("Failed to remove expired GitHub session: {error}");
                        }
                        router.token = None;
                        router.user = None;
                        router.notifications.clear();
                        router.notifications_loading = false;
                        router.current_screen = Screen::Auth;
                        router.auth_phase = AuthPhase::Error(
                            "Your GitHub session expired. Sign in again to continue.".to_string(),
                        );
                        cx.notify();
                        return;
                    }

                    let user_failed = match user_result {
                        Ok(user) => {
                            router.user = Some(user);
                            false
                        }
                        Err(error) => {
                            log::warn!("Failed to load GitHub profile: {error:#}");
                            true
                        }
                    };

                    let notifications_failed = match notifs_result {
                        Ok(notifications) => {
                            router.notifications = notifications;
                            false
                        }
                        Err(error) => {
                            log::warn!("Failed to load GitHub notifications: {error:#}");
                            true
                        }
                    };

                    router.home_error = match (user_failed, notifications_failed) {
                    (_, true) => Some(
                        "Notifications could not be refreshed. Check your connection and try again."
                            .to_string(),
                    ),
                    (true, false) => Some(
                        "Notifications loaded, but your profile is temporarily unavailable."
                            .to_string(),
                    ),
                    (false, false) => None,
                };
                    router.notifications_loading = false;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub fn refresh_notifications(&mut self, cx: &mut gpui::Context<Self>) {
        if let Some(token) = self.token.clone() {
            self.load_home_data(cx, token);
        }
    }

    pub fn open_notification(&mut self, notification: Notification, cx: &mut gpui::Context<Self>) {
        gpui_mobile::set_text_input_callback(None);
        gpui_mobile::hide_keyboard();

        self.selected_notification = Some(notification.clone());
        self.notification_detail = None;
        self.notification_detail_error = None;
        self.notification_detail_loading = true;
        self.detail_request_id = self.detail_request_id.wrapping_add(1);
        let request_id = self.detail_request_id;
        self.navigate_to(Screen::Detail);
        cx.notify();

        let Some(token) = self.token.clone() else {
            self.notification_detail_loading = false;
            self.notification_detail_error = Some("GitHub session is unavailable.".to_string());
            cx.notify();
            return;
        };
        let Some(detail_url) =
            crate::api::notification_detail_url(&notification).map(str::to_owned)
        else {
            self.notification_detail_loading = false;
            self.notification_detail_error =
                Some("GitHub did not provide details for this notification.".to_string());
            cx.notify();
            return;
        };

        cx.spawn(async move |entity: gpui::WeakEntity<Self>, cx| {
            let result = match crate::api::make_client() {
                Ok(client) => {
                    crate::api::get_notification_detail(&client, &token, &detail_url).await
                }
                Err(error) => Err(error),
            };

            entity
                .update(cx, |router, cx| {
                    if router.detail_request_id != request_id {
                        return;
                    }

                    router.notification_detail_loading = false;
                    match result {
                        Ok(detail) => router.notification_detail = Some(detail),
                        Err(error) => {
                            log::warn!("Failed to load notification detail: {error:#}");
                            router.notification_detail_error = Some(
                                "The notification opened, but its extra details could not be loaded."
                                    .to_string(),
                            );
                        }
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    pub fn close_notification(&mut self, cx: &mut gpui::Context<Self>) {
        self.detail_request_id = self.detail_request_id.wrapping_add(1);
        self.navigate_to(Screen::Home);
        self.selected_notification = None;
        self.notification_detail = None;
        self.notification_detail_loading = false;
        self.notification_detail_error = None;
        cx.notify();
    }

    pub fn open_selected_notification_in_github(&self) {
        let Some(notification) = self.selected_notification.as_ref() else {
            return;
        };
        let detail_url = self
            .notification_detail
            .as_ref()
            .and_then(|detail| detail.html_url.clone());
        let url = match (detail_url, notification.subject.kind.as_str()) {
            (Some(url), _) => url,
            (None, "Release" | "CheckSuite") => return,
            (None, _) => crate::api::notification_web_url(notification),
        };

        match gpui_mobile::packages::url_launcher::launch_url(&url) {
            Ok(true) => {}
            Ok(false) => log::warn!("No app could open notification URL"),
            Err(error) => log::warn!("Failed to open notification URL: {error}"),
        }
    }

    pub fn sign_out(&mut self, cx: &mut gpui::Context<Self>) {
        self.auth_request_id = self.auth_request_id.wrapping_add(1);
        if let Some(cancel) = self.polling_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        self.home_request_id = self.home_request_id.wrapping_add(1);
        self.detail_request_id = self.detail_request_id.wrapping_add(1);
        if let Err(error) = crate::credentials::clear_token() {
            log::warn!("Failed to remove saved GitHub session: {error}");
        }
        self.token = None;
        self.user = None;
        self.notifications.clear();
        self.notifications_loading = false;
        self.home_error = None;
        self.search_query.clear();
        self.selected_notification = None;
        self.notification_detail = None;
        self.notification_detail_loading = false;
        self.notification_detail_error = None;
        self.auth_phase = AuthPhase::Idle;
        self.navigate_to(Screen::Auth);
        gpui_mobile::set_text_input_callback(None);
        gpui_mobile::hide_keyboard();
        cx.notify();
    }
}

pub fn safe_area_insets() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "android")]
    {
        if let Some(window) =
            gpui_mobile::android::jni::platform().and_then(|platform| platform.primary_window())
        {
            let insets = window.safe_area_insets_logical();
            return (insets.top, insets.bottom, insets.left, insets.right);
        }
    }

    gpui_mobile::safe_area_insets()
}

pub fn device_authorization_url(verification_uri: &str, user_code: &str) -> String {
    let separator = if verification_uri.contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{verification_uri}{separator}act4g_code={user_code}")
}

impl gpui::Render for Router {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        set_system_chrome(&SystemChromeStyle {
            status_bar_style: StatusBarContentStyle::Light,
            ..Default::default()
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(gpui::rgb(0x0D1117))
            .child(match self.current_screen {
                Screen::Auth => auth::render(self, cx).into_any_element(),
                Screen::Home => home::render(self, cx).into_any_element(),
                Screen::Detail => detail::render(self, cx).into_any_element(),
            })
    }
}
