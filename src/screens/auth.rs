use gpui::{div, prelude::*, px, rgb};

use super::{AuthPhase, Router};

const BG: u32 = 0x0D1117;
const SURFACE: u32 = 0x161B22;
const BORDER: u32 = 0x30363D;
const TEXT: u32 = 0xC9D1D9;
const SUBTEXT: u32 = 0x8B949E;
const ACCENT: u32 = 0x58A6FF;
const RED: u32 = 0xF85149;
const CODE_BG: u32 = 0x21262D;

pub fn render(router: &Router, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let (safe_top, safe_bottom, _, _) = gpui_mobile::safe_area_insets();

    div()
        .flex()
        .flex_col()
        .size_full()
        .items_center()
        .justify_center()
        .pt(px(safe_top))
        .pb(px(safe_bottom))
        .px_8()
        .gap_8()
        // Logo + app title
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(72.))
                        .rounded_2xl()
                        .bg(rgb(SURFACE))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_3xl()
                        .child(""),
                )
                .child(
                    div()
                        .text_3xl()
                        .text_color(rgb(TEXT))
                        .child("act4g"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(SUBTEXT))
                        .child("A GitHub client"),
                ),
        )
        // Auth phase content
        .child(match &router.auth_phase {
            AuthPhase::Idle => idle_view(cx).into_any_element(),
            AuthPhase::RequestingCode => spinner_view("Requesting device code…").into_any_element(),
            AuthPhase::WaitingForUser {
                user_code,
                verification_uri,
                ..
            } => waiting_view(user_code, verification_uri, cx).into_any_element(),
            AuthPhase::Polling => spinner_view("Completing sign in…").into_any_element(),
            AuthPhase::Error(msg) => error_view(msg, cx).into_any_element(),
        })
}

fn idle_view(cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .child(
            div()
                .w_full()
                .h(px(52.))
                .rounded_lg()
                .bg(rgb(TEXT))
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
                        .text_color(rgb(BG))
                        .child("Sign in with GitHub"),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(SUBTEXT))
                .text_center()
                .child("Opens GitHub device authorization in your browser"),
        )
}

fn spinner_view(msg: &str) -> impl gpui::IntoElement {
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
                .border_color(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_xl()
                .child("⟳"),
        )
        .child(div().text_sm().text_color(rgb(SUBTEXT)).child(msg.to_string()))
}

fn waiting_view(
    user_code: &str,
    verification_uri: &str,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let code = user_code.to_string();
    let uri = verification_uri.to_string();

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
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(BORDER))
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(SUBTEXT))
                        .child("1. Open the URL below in your browser"),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(rgb(CODE_BG))
                        .text_sm()
                        .text_color(rgb(ACCENT))
                        .child(uri.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(SUBTEXT))
                        .child("2. Enter this code"),
                )
                // Big code display
                .child(
                    div()
                        .w_full()
                        .py_4()
                        .rounded_xl()
                        .bg(rgb(CODE_BG))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_3xl()
                                .text_color(rgb(TEXT))
                                .child(code.clone()),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(SUBTEXT))
                        .text_center()
                        .child("Waiting for authorization…"),
                ),
        )
        // Cancel button
        .child(
            div()
                .w_full()
                .h(px(44.))
                .rounded_lg()
                .border_1()
                .border_color(rgb(BORDER))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| {
                        if let Some(cancel) = router.polling_cancel.take() {
                            cancel.store(true, std::sync::atomic::Ordering::Release);
                        }
                        router.auth_phase = AuthPhase::Idle;
                        cx.notify();
                    }),
                )
                .child(div().text_sm().text_color(rgb(SUBTEXT)).child("Cancel")),
        )
}

fn error_view(msg: &str, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
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
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(RED))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(RED))
                        .child("Authentication failed"),
                )
                .child(div().text_xs().text_color(rgb(SUBTEXT)).child(msg)),
        )
        .child(
            div()
                .w_full()
                .h(px(52.))
                .rounded_lg()
                .bg(rgb(TEXT))
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
                .child(div().text_base().text_color(rgb(BG)).child("Try again")),
        )
}
