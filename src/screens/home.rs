use gpui::{div, prelude::*, px, rgb, SharedString};
use std::cell::RefCell;

use super::{safe_area_insets, Router};
use crate::api::Notification;

const BG: u32 = 0x0D1117;
const SURFACE: u32 = 0x161B22;
const BORDER: u32 = 0x21262D;
const TEXT: u32 = 0xC9D1D9;
const SUBTEXT: u32 = 0x8B949E;
const ACCENT: u32 = 0x58A6FF;
const GREEN: u32 = 0x3FB950;
const PURPLE: u32 = 0xBC8CFF;
const YELLOW: u32 = 0xF0883E;

// Search field text, accumulated from keyboard callbacks each frame.
thread_local! {
    static PENDING_TEXT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
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

    let (safe_top, safe_bottom, safe_left, safe_right) = safe_area_insets();
    let search_focused = !router.search_query.is_empty()
        || gpui_mobile::TEXT_INPUT_DIRTY.load(std::sync::atomic::Ordering::Acquire);

    let filtered = filter_notifications(&router.notifications, &router.search_query);
    let home_error = router.home_error.clone();
    let is_searching = !router.search_query.is_empty();

    div()
        .flex()
        .flex_col()
        .size_full()
        .pt(px(safe_top))
        .pb(px(safe_bottom))
        .pl(px(safe_left))
        .pr(px(safe_right))
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
                .children(home_error.as_ref().and_then(|message| {
                    if router.notifications.is_empty() {
                        None
                    } else {
                        Some(error_banner(message, cx).into_any_element())
                    }
                }))
                .child(
                    if router.notifications_loading && router.notifications.is_empty() {
                        loading_placeholder().into_any_element()
                    } else if let Some(message) = home_error
                        .as_ref()
                        .filter(|_| router.notifications.is_empty())
                    {
                        error_state(message, cx).into_any_element()
                    } else if filtered.is_empty() {
                        empty_inbox(is_searching).into_any_element()
                    } else {
                        inbox_list(&filtered, router.notifications_loading, cx).into_any_element()
                    },
                ),
        )
}

fn top_bar(
    router: &Router,
    search_focused: bool,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let query = router.search_query.clone();
    let placeholder = if query.is_empty() {
        "Search notifications"
    } else {
        ""
    };
    let display_text = if query.is_empty() {
        placeholder.to_string()
    } else {
        query.clone()
    };
    let display_color = if query.is_empty() { SUBTEXT } else { TEXT };

    let account_label = router
        .user
        .as_ref()
        .map(|user| format!("@{}", user.login))
        .unwrap_or_else(|| "GitHub notifications".to_string());

    // Avatar: remote image when available, otherwise the account initial.
    let (initials, has_avatar, avatar_url) = match &router.user {
        Some(u) => {
            let init = u
                .login
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".to_string());
            (init, !u.avatar_url.is_empty(), u.avatar_url.clone())
        }
        None => ("?".to_string(), false, String::new()),
    };

    div()
        .flex()
        .flex_col()
        .gap_4()
        .px_5()
        .pt_3()
        .pb_4()
        .border_b_1()
        .border_color(rgb(BORDER))
        // Page title and account identity.
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xl().text_color(rgb(TEXT)).child("Inbox"))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .child(account_label),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .rounded_lg()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|router, _, _, cx| router.sign_out(cx)),
                                )
                                .child("Sign out"),
                        )
                        .child(if has_avatar {
                            div()
                                .size(px(40.))
                                .rounded_full()
                                .overflow_hidden()
                                .border_1()
                                .border_color(rgb(BORDER))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    gpui::img(SharedString::from(avatar_url))
                                        .size(px(40.))
                                        .rounded_full(),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .size(px(40.))
                                .rounded_full()
                                .bg(rgb(ACCENT))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(rgb(BG))
                                .child(initials)
                                .into_any_element()
                        }),
                ),
        )
        // Search field.
        .child(
            div()
                .w_full()
                .h(px(44.))
                .px_4()
                .rounded_lg()
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(if search_focused {
                    rgb(ACCENT)
                } else {
                    rgb(BORDER)
                })
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
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(rgb(display_color))
                        .child(display_text),
                )
                .children(if !query.is_empty() {
                    vec![div()
                        .text_xs()
                        .text_color(rgb(SUBTEXT))
                        .px_2()
                        .py_1()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|router, _, _, cx| {
                                router.search_query.clear();
                                gpui_mobile::hide_keyboard();
                                cx.notify();
                            }),
                        )
                        .child("Clear")
                        .into_any_element()]
                } else {
                    vec![]
                }),
        )
}

fn inbox_list(
    notifications: &[&Notification],
    loading: bool,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
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
                        .h(px(36.))
                        .px_3()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .items_center()
                        .text_sm()
                        .text_color(if loading { rgb(SUBTEXT) } else { rgb(ACCENT) })
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|router, _, _, cx| {
                                router.refresh_notifications(cx);
                            }),
                        )
                        .child(if loading { "Refreshing..." } else { "Refresh" }),
                ),
        )
        // Notification items
        .children(notifications.iter().map(|n| notif_item(n, cx)))
}

fn notif_item(notif: &Notification, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let (type_color, type_label) = notif_type_style(&notif.subject.kind);
    let date = format_date(&notif.updated_at);
    let repo = notif.repository.full_name.clone();
    let title = notif.subject.title.clone();
    let reason = notif.reason.clone();
    let notification = notif.clone();

    div()
        .id(SharedString::from(format!("notification-{}", notif.id)))
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_3()
        .mb_3()
        .p_4()
        .rounded_xl()
        .overflow_hidden()
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(if notif.unread {
            rgb(ACCENT)
        } else {
            rgb(BORDER)
        })
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |router, _, _, cx| {
                router.open_notification(notification.clone(), cx);
            }),
        )
        // Repository, type, and date.
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(30.))
                                .h(px(22.))
                                .rounded(px(5.))
                                .bg(gpui::rgba(type_color * 256 + 0x26))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .text_color(rgb(type_color))
                                .child(type_label),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .truncate()
                                .child(repo),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(rgb(SUBTEXT))
                        .child(date),
                ),
        )
        // Two-line title that cannot force the row wider than the screen.
        .child(
            div()
                .min_w_0()
                .text_base()
                .text_color(rgb(TEXT))
                .line_clamp(2)
                .child(title),
        )
        // Reason and affordance.
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .children(if notif.unread {
                            vec![div()
                                .size(px(7.))
                                .rounded_full()
                                .bg(rgb(ACCENT))
                                .into_any_element()]
                        } else {
                            vec![]
                        })
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .child(reason_label(&reason)),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(ACCENT))
                        .child("View details  →"),
                ),
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
        .child(
            div()
                .size(px(48.))
                .rounded_full()
                .border_2()
                .border_color(rgb(ACCENT))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(ACCENT))
                .child("..."),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(SUBTEXT))
                .child("Loading notifications…"),
        )
}

fn empty_inbox(is_searching: bool) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .gap_3()
        .py_16()
        .child(
            div()
                .size(px(64.))
                .rounded_full()
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(BORDER))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(GREEN))
                .child(if is_searching { "0" } else { "OK" }),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(TEXT))
                .child(if is_searching {
                    "No matches"
                } else {
                    "You're all caught up"
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(SUBTEXT))
                .text_center()
                .child(if is_searching {
                    "Try another repository or notification title"
                } else {
                    "New GitHub notifications will appear here"
                }),
        )
}

fn error_banner(message: &str, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let message = message.to_string();
    div()
        .mx_4()
        .mt_4()
        .p_3()
        .rounded_lg()
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(YELLOW))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(rgb(TEXT))
                .child(message),
        )
        .child(
            div()
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(rgb(TEXT))
                .text_xs()
                .text_color(rgb(BG))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| router.refresh_notifications(cx)),
                )
                .child("Retry"),
        )
}

fn error_state(message: &str, cx: &mut gpui::Context<Router>) -> impl gpui::IntoElement {
    let message = message.to_string();
    div()
        .flex_1()
        .px_8()
        .py_16()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_4()
        .child(
            div()
                .size(px(64.))
                .rounded_full()
                .bg(rgb(SURFACE))
                .border_1()
                .border_color(rgb(YELLOW))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(YELLOW))
                .child("!"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(TEXT))
                .child("Couldn't load notifications"),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(SUBTEXT))
                .text_center()
                .child(message),
        )
        .child(
            div()
                .h(px(44.))
                .px_6()
                .rounded_lg()
                .bg(rgb(TEXT))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(BG))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| router.refresh_notifications(cx)),
                )
                .child("Try again"),
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
        _ => (SUBTEXT, "N"),
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
    // Compact month/day form from "YYYY-MM-DDTHH:MM:SSZ".
    iso.get(5..10).unwrap_or(iso).replace('-', "/")
}
