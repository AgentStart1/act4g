use super::theme::Palette;
use super::{device_authorization_url, safe_area_insets, AuthPhase, Router};
use gpui::{div, prelude::*, px, rgb};
use std::sync::{Arc, OnceLock};

fn app_icon() -> Arc<gpui::Image> {
    static ICON: OnceLock<Arc<gpui::Image>> = OnceLock::new();
    ICON.get_or_init(|| {
        Arc::new(gpui::Image::from_bytes(
            gpui::ImageFormat::Png,
            include_bytes!("../../android/gradle/app/src/main/res/mipmap-xxxhdpi/ic_launcher.png")
                .to_vec(),
        ))
    })
    .clone()
}

pub fn render(router: &Router, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let palette = router.palette;
    let (safe_top, safe_bottom, safe_left, safe_right) = safe_area_insets();

    div()
        .id("authorization-scroll")
        .flex()
        .flex_col()
        .size_full()
        .items_center()
        .overflow_y_scroll()
        .pt(px(safe_top + 32.))
        .pb(px(safe_bottom + 32.))
        .pl(px(safe_left + 32.))
        .pr(px(safe_right + 32.))
        .gap_8()
        // Logo + app title
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_3()
                .child(
                    gpui::img(app_icon())
                        .size(px(72.))
                        .rounded_2xl()
                        .object_fit(gpui::ObjectFit::Contain),
                )
                .child(div().text_3xl().text_color(rgb(palette.text)).child("gact"))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.subtext))
                        .child("Your GitHub, at a glance"),
                ),
        )
        // Auth phase content
        .child(match &router.auth_phase {
            AuthPhase::Idle => idle_view(palette, cx).into_any_element(),
            AuthPhase::RequestingCode => {
                spinner_view(palette, "Requesting device code…").into_any_element()
            }
            AuthPhase::WaitingForUser {
                user_code,
                verification_uri,
                ..
            } => waiting_view(palette, user_code, verification_uri, cx).into_any_element(),
            AuthPhase::Polling => spinner_view(palette, "Completing sign in…").into_any_element(),
            AuthPhase::Error(msg) => error_view(palette, msg, cx).into_any_element(),
        })
}

fn idle_view(palette: Palette, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .child(
            div()
                .w_full()
                .h(px(52.))
                .rounded_xl()
                .bg(rgb(palette.accent))
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| {
                        router.start_device_flow(cx);
                    }),
                )
                .child(
                    div()
                        .text_base()
                        .text_color(rgb(palette.on_accent))
                        .child("Sign in with GitHub"),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.subtext))
                .text_center()
                .child("Opens GitHub device authorization in your browser"),
        )
}

fn spinner_view(palette: Palette, msg: &str) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_4()
        .child(
            div()
                .size(px(32.))
                .rounded_full()
                .border_2()
                .border_color(rgb(palette.accent))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(palette.accent))
                .child("..."),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(palette.subtext))
                .child(msg.to_string()),
        )
}

fn waiting_view(
    palette: Palette,
    user_code: &str,
    verification_uri: &str,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let code = user_code.to_string();
    let uri = verification_uri.to_string();
    let open_uri = device_authorization_url(&uri, &code);

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_5()
        // Instruction card
        .child(
            div()
                .w_full()
                .p_5()
                .rounded_xl()
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(rgb(palette.border))
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.subtext))
                        .child("1. Open GitHub in your browser"),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_xl()
                        .bg(rgb(palette.code_bg))
                        .text_sm()
                        .text_color(rgb(palette.accent))
                        .child(uri.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.subtext))
                        .child("2. Enter this code"),
                )
                // Big code display
                .child(
                    div()
                        .w_full()
                        .py_4()
                        .rounded_xl()
                        .bg(rgb(palette.code_bg))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_3xl()
                                .text_color(rgb(palette.text))
                                .child(code.clone()),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(44.))
                        .rounded_xl()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |_, _, _, _| {
                                if let Err(error) =
                                    gpui_mobile::packages::clipboard::set_text(&code)
                                {
                                    log::warn!("Failed to copy device authorization code: {error}");
                                }
                            }),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(palette.accent))
                                .child("Copy code"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.subtext))
                        .text_center()
                        .child("Waiting for authorization…"),
                ),
        )
        // Re-open authorization if the browser did not navigate correctly.
        .child(
            div()
                .w_full()
                .h(px(52.))
                .rounded_xl()
                .bg(rgb(palette.accent))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |_router, _, _, _cx| {
                        match gpui_mobile::packages::url_launcher::launch_url(&open_uri) {
                            Ok(true) => {}
                            Ok(false) => {
                                log::warn!("No app could open the GitHub authorization URL")
                            }
                            Err(error) => {
                                log::warn!("Failed to open GitHub authorization URL: {error}")
                            }
                        }
                    }),
                )
                .child(
                    div()
                        .text_base()
                        .text_color(rgb(palette.on_accent))
                        .child("Open GitHub"),
                ),
        )
        // Cancel button
        .child(
            div()
                .w_full()
                .h(px(44.))
                .rounded_xl()
                .border_1()
                .border_color(rgb(palette.border))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| {
                        router.cancel_device_flow(cx);
                    }),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.subtext))
                        .child("Cancel"),
                ),
        )
}

fn error_view(
    palette: Palette,
    msg: &str,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let msg = msg.to_string();
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_4()
        .child(
            div()
                .w_full()
                .p_4()
                .rounded_xl()
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(rgb(palette.red))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(palette.red))
                        .child("Authentication failed"),
                )
                .child(div().text_xs().text_color(rgb(palette.subtext)).child(msg)),
        )
        .child(
            div()
                .w_full()
                .h(px(52.))
                .rounded_xl()
                .bg(rgb(palette.accent))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| {
                        router.auth_phase = AuthPhase::Idle;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .text_base()
                        .text_color(rgb(palette.on_accent))
                        .child("Try again"),
                ),
        )
}
