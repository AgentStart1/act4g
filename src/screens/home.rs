use gpui::{div, prelude::*, px, rgb, SharedString};
use std::cell::RefCell;

use super::Router;
use crate::api::Notification;

const BG: u32 = 0x0D1117;
const SURFACE: u32 = 0x161B22;
const BORDER: u32 = 0x21262D;
const TEXT: u32 = 0xC9D1D9;
const SUBTEXT: u32 = 0x8B949E;
const ACCENT: u32 = 0x58A6FF;
const GREEN: u32 = 0x3FB950;
const RED: u32 = 0xF85149;
const PURPLE: u32 = 0xBC8CFF;
const YELLOW: u32 = 0xF0883E;

// Search field text, accumulated from keyboard callbacks each frame.
thread_local! {
    static PENDING_TEXT: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

/// Install the soft keyboard callback that routes typed characters here.
fn install_keyboard_callback() {
    gpui_mobile::set_text_input_callback(Some(Box::new(|text: &str| {
        PENDING_TEXT.with(|p| p.borrow_mut().push(text.to_string()));
    })));
}

pub fn render(router: &mut Router, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    // Drain any pending keyboard input into the search query.
    PENDING_TEXT.with(|p| {
        let mut pending = p.borrow_mut();
        for fragment in pending.drain(..) {
            if fragment == "\x08" {
                // Backspace
                router.search_query.pop();
            } else {
                router.search_query.push_str(&fragment);
            }
        }
        if !pending.is_empty() {
            // pending was drained, so this branch never fires; just guard.
        }
    });

    let (safe_top, safe_bottom, _, _) = gpui_mobile::safe_area_insets();
    let search_focused = !router.search_query.is_empty()
        || gpui_mobile::TEXT_INPUT_DIRTY.load(std::sync::atomic::Ordering::Acquire);

    let filtered = filter_notifications(&router.notifications, &router.search_query);

    div()
        .flex()
        .flex_col()
        .size_full()
        .pt(px(safe_top))
        .pb(px(safe_bottom))
        // ── Top bar ──────────────────────────────────────────────────────
        .child(top_bar(router, search_focused, cx))
        // ── Inbox list ───────────────────────────────────────────────────
        .child(
            div()
                .id("inbox-scroll")
                .flex_1()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .child(if router.notifications_loading && router.notifications.is_empty() {
                    loading_placeholder().into_any_element()
                } else if filtered.is_empty() {
                    empty_inbox(router.notifications_loading).into_any_element()
                } else {
                    inbox_list(&filtered, cx).into_any_element()
                }),
        )
}

fn top_bar(router: &Router, search_focused: bool, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let query = router.search_query.clone();
    let placeholder = if query.is_empty() { "Search…" } else { "" };
    let display_text = if query.is_empty() {
        placeholder.to_string()
    } else {
        query.clone()
    };
    let display_color = if query.is_empty() { SUBTEXT } else { TEXT };

    // Avatar: initials or first letter of login
    let (initials, has_avatar, avatar_url) = match &router.user {
        Some(u) => {
            let init = u.login.chars().next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".to_string());
            (init, !u.avatar_url.is_empty(), u.avatar_url.clone())
        }
        None => ("?".to_string(), false, String::new()),
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_3()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(rgb(BORDER))
        // Search bar (flex-1)
        .child(
            div()
                .flex_1()
                .h(px(40.))
                .px_4()
                .rounded_full()
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(if search_focused { rgb(ACCENT) } else { rgb(BORDER) })
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|_router, _, _, _cx| {
                        install_keyboard_callback();
                        gpui_mobile::show_keyboard_with_type(gpui_mobile::KeyboardType::Default);
                    }),
                )
                .child(div().text_sm().text_color(rgb(SUBTEXT)).child("🔍"))
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(rgb(display_color))
                        .child(display_text),
                )
                .children(if !query.is_empty() {
                    vec![
                        div()
                            .text_xs()
                            .text_color(rgb(SUBTEXT))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|router, _, _, cx| {
                                    router.search_query.clear();
                                    gpui_mobile::hide_keyboard();
                                    cx.notify();
                                }),
                            )
                            .child("✕")
                            .into_any_element()
                    ]
                } else {
                    vec![]
                }),
        )
        // Avatar
        .child(if has_avatar {
            div()
                .size(px(36.))
                .rounded_full()
                .overflow_hidden()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    gpui::img(SharedString::from(avatar_url))
                        .size(px(36.))
                        .rounded_full(),
                )
                .into_any_element()
        } else {
            div()
                .size(px(36.))
                .rounded_full()
                .bg(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(BG))
                .child(initials)
                .into_any_element()
        })
}

fn inbox_list(notifications: &[&Notification], cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .px_4()
        .pt_2()
        // Section header
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .py_3()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(div().text_base().text_color(rgb(TEXT)).child("Inbox"))
                        .child(
                            div()
                                .px_2()
                                .py(px(2.))
                                .rounded_full()
                                .bg(rgb(ACCENT))
                                .text_xs()
                                .text_color(rgb(BG))
                                .child(notifications.len().to_string()),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(SUBTEXT))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|router, _, _, cx| {
                                router.refresh_notifications(cx);
                            }),
                        )
                        .child("↺ Refresh"),
                ),
        )
        // Notification items
        .children(notifications.iter().map(|n| notif_item(n)))
}

fn notif_item(notif: &Notification) -> impl gpui::IntoElement {
    let (type_color, type_label) = notif_type_style(&notif.subject.kind);
    let date = format_date(&notif.updated_at);
    let repo = notif.repository.full_name.clone();
    let title = notif.subject.title.clone();
    let reason = notif.reason.clone();

    div()
        .flex()
        .flex_row()
        .gap_3()
        .py_3()
        .border_b_1()
        .border_color(rgb(BORDER))
        .items_start()
        // Type badge
        .child(
            div()
                .w(px(28.))
                .h(px(20.))
                .mt(px(2.))
                .rounded(px(4.))
                .bg(rgb(type_color).opacity(0.15))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(type_color))
                        .child(type_label),
                ),
        )
        // Text content
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .child(repo),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .child(date),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(SUBTEXT))
                        .child(reason_label(&reason)),
                ),
        )
        // Unread dot
        .child(
            div()
                .size(px(8.))
                .mt(px(6.))
                .rounded_full()
                .bg(rgb(ACCENT))
                .opacity(if notif.unread { 1.0 } else { 0.0 }),
        )
}

fn loading_placeholder() -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .gap_3()
        .py_16()
        .child(div().text_3xl().child("⟳"))
        .child(div().text_sm().text_color(rgb(SUBTEXT)).child("Loading notifications…"))
}

fn empty_inbox(loading: bool) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .gap_3()
        .py_16()
        .child(div().text_3xl().child(if loading { "⟳" } else { "✓" }))
        .child(
            div()
                .text_sm()
                .text_color(rgb(SUBTEXT))
                .child(if loading { "Loading…" } else { "All caught up!" }),
        )
}

fn filter_notifications<'a>(
    notifications: &'a [Notification],
    query: &str,
) -> Vec<&'a Notification> {
    if query.is_empty() {
        return notifications.iter().collect();
    }
    let q = query.to_ascii_lowercase();
    notifications
        .iter()
        .filter(|n| {
            n.subject.title.to_ascii_lowercase().contains(&q)
                || n.repository.full_name.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

fn notif_type_style(kind: &str) -> (u32, &'static str) {
    match kind {
        "PullRequest" => (PURPLE, "PR"),
        "Issue" => (GREEN, "#"),
        "Release" => (ACCENT, "R"),
        "CheckSuite" => (YELLOW, "CI"),
        _ => (SUBTEXT, "·"),
    }
}

fn reason_label(reason: &str) -> &'static str {
    match reason {
        "assign" => "Assigned",
        "author" => "You authored",
        "comment" => "New comment",
        "mention" => "You were mentioned",
        "review_requested" => "Review requested",
        "subscribed" => "Subscribed",
        "team_mention" => "Team mention",
        "ci_activity" => "CI activity",
        _ => "Notification",
    }
}

fn format_date(iso: &str) -> String {
    // Show "YYYY-MM-DD" from "YYYY-MM-DDTHH:MM:SSZ"
    iso.get(0..10).unwrap_or(iso).to_string()
}
