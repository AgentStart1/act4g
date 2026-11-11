use super::theme::Palette;
use gpui::{div, prelude::*, px, rgb, SharedString};
use std::cell::RefCell;

use super::{safe_area_insets, Router};
use crate::api::Notification;

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
    let palette = router.palette;
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

    let filtered = filter_notifications(
        &router.notifications,
        &router.search_query,
        router.unread_only,
    );
    let home_error = router.home_error.clone();
    let is_searching = !router.search_query.is_empty() || router.unread_only;

    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .bg(rgb(palette.bg))
        .pt(px(safe_top))
        .pb(px(safe_bottom))
        .pl(px(safe_left))
        .pr(px(safe_right))
        // ── Top bar ──────────────────────────────────────────────────────
        .child(top_bar(palette, router, search_focused, cx))
        .child(inbox_filters(palette, router, cx))
        // ── Inbox list ───────────────────────────────────────────────────
        .child(
            div()
                .id("inbox-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&router.scroll_positions.inbox)
                .flex()
                .flex_col()
                .children(home_error.as_ref().and_then(|message| {
                    if router.notifications.is_empty() {
                        None
                    } else {
                        Some(error_banner(palette, message, cx).into_any_element())
                    }
                }))
                .child(
                    if router.notifications_loading && router.notifications.is_empty() {
                        loading_placeholder(palette).into_any_element()
                    } else if let Some(message) = home_error
                        .as_ref()
                        .filter(|_| router.notifications.is_empty())
                    {
                        error_state(palette, message, cx).into_any_element()
                    } else if filtered.is_empty() {
                        empty_inbox(palette, is_searching, router.keyboard_bottom_inset > 0.)
                            .into_any_element()
                    } else {
                        inbox_list(palette, &filtered, router.notifications_loading, cx)
                            .into_any_element()
                    },
                ),
        )
        .when(router.keyboard_bottom_inset <= 0., |view| {
            view.child(super::navigation::render(palette, super::Screen::Home, cx))
        })
}

fn inbox_filters(
    palette: Palette,
    router: &Router,
    cx: &mut gpui::Context<Router>,
) -> impl IntoElement {
    let unread = router.notifications.iter().filter(|n| n.unread).count();
    div().flex().gap_2().px_5().py_3().children(
        [
            (false, format!("All · {}", router.notifications.len())),
            (true, format!("Unread · {unread}")),
        ]
        .into_iter()
        .map(|(unread_only, label)| {
            let selected = router.unread_only == unread_only;
            div()
                .id(if unread_only {
                    "filter-unread"
                } else {
                    "filter-all"
                })
                .flex_1()
                .min_w_0()
                .h(px(48.))
                .rounded_xl()
                .bg(rgb(if selected {
                    palette.accent
                } else {
                    palette.surface
                }))
                .text_color(rgb(if selected {
                    palette.on_accent
                } else {
                    palette.subtext
                }))
                .text_sm()
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |router, _, _, cx| {
                        router.unread_only = unread_only;
                        cx.notify();
                    }),
                )
                .child(label)
        }),
    )
}

fn top_bar(
    palette: Palette,
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
    let display_color = if query.is_empty() {
        palette.subtext
    } else {
        palette.text
    };

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
        .border_color(rgb(palette.border))
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
                        .child(
                            div()
                                .text_3xl()
                                .text_color(rgb(palette.text))
                                .child("Activity"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.subtext))
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
                                .min_h(px(48.))
                                .flex()
                                .items_center()
                                .rounded_xl()
                                .text_xs()
                                .text_color(rgb(palette.subtext))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(|router, _, _, cx| router.sign_out(cx)),
                                )
                                .child("Sign out"),
                        )
                        .child(if has_avatar {
                            div()
                                .size(px(48.))
                                .rounded_full()
                                .overflow_hidden()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    gpui::img(SharedString::from(avatar_url))
                                        .size(px(48.))
                                        .rounded_full(),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .size(px(48.))
                                .rounded_full()
                                .bg(rgb(palette.accent))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(rgb(palette.bg))
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
                .rounded_xl()
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(if search_focused {
                    rgb(palette.accent)
                } else {
                    rgb(palette.border)
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
                        .text_color(rgb(palette.subtext))
                        .px_2()
                        .py_3()
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
    palette: Palette,
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
                        .child(
                            div()
                                .text_base()
                                .text_color(rgb(palette.text))
                                .child("Activity"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py(px(2.))
                                .rounded_full()
                                .bg(rgb(palette.accent))
                                .text_xs()
                                .text_color(rgb(palette.bg))
                                .child(notifications.len().to_string()),
                        ),
                )
                .child(
                    div()
                        .h(px(48.))
                        .px_3()
                        .rounded_xl()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .flex()
                        .items_center()
                        .text_sm()
                        .text_color(if loading {
                            rgb(palette.subtext)
                        } else {
                            rgb(palette.accent)
                        })
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
        .children(notifications.iter().map(|n| notif_item(palette, n, cx)))
}

fn notif_item(
    palette: Palette,
    notif: &Notification,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let (type_color, type_label) = notif_type_style(palette, &notif.subject.kind);
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
        .gap_2()
        .py_4()
        .px_1()
        .flex_shrink_0()
        .overflow_hidden()
        .bg(rgb(palette.bg))
        .border_b_1()
        .border_color(rgb(palette.border))
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
                                .text_color(rgb(palette.subtext))
                                .truncate()
                                .child(repo),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(rgb(palette.subtext))
                        .child(date),
                ),
        )
        // Two-line title that cannot force the row wider than the screen.
        .child(
            div()
                .min_w_0()
                .text_base()
                .font_weight(if notif.unread {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(rgb(if notif.unread {
                    palette.text
                } else {
                    palette.subtext
                }))
                .line_clamp(2)
                .text_ellipsis()
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
                                .size(px(8.))
                                .rounded_full()
                                .bg(rgb(palette.accent))
                                .into_any_element()]
                        } else {
                            vec![]
                        })
                        .child(div().text_xs().text_color(rgb(palette.subtext)).child(
                            if notif.unread {
                                format!("Unread · {}", reason_label(&reason))
                            } else {
                                reason_label(&reason).to_string()
                            },
                        )),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.accent))
                        .child("View details  →"),
                ),
        )
}

fn loading_placeholder(palette: Palette) -> impl gpui::IntoElement {
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
                .child("Loading notifications…"),
        )
}

fn empty_inbox(palette: Palette, is_searching: bool, compact: bool) -> impl gpui::IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .gap_3()
        .px_4()
        .py(px(if compact { 12. } else { 48. }))
        .flex_shrink_0()
        .child(
            div()
                .size(px(if compact { 32. } else { 64. }))
                .rounded_full()
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(rgb(palette.border))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(palette.green))
                .child(if is_searching { "0" } else { "OK" }),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(palette.text))
                .child(if is_searching {
                    "Nothing here yet"
                } else {
                    "You're all caught up"
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.subtext))
                .text_center()
                .child(if is_searching {
                    "Switch to All activity or try another search"
                } else {
                    "New GitHub notifications will appear here"
                }),
        )
}

fn error_banner(
    palette: Palette,
    message: &str,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
    let message = message.to_string();
    div()
        .mx_4()
        .mt_4()
        .p_3()
        .rounded_xl()
        .bg(rgb(palette.surface))
        .border_1()
        .border_color(rgb(palette.yellow))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(rgb(palette.text))
                .child(message),
        )
        .child(
            div()
                .px_3()
                .py_2()
                .rounded_xl()
                .bg(rgb(palette.text))
                .text_xs()
                .text_color(rgb(palette.bg))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|router, _, _, cx| router.refresh_notifications(cx)),
                )
                .child("Retry"),
        )
}

fn error_state(
    palette: Palette,
    message: &str,
    cx: &mut gpui::Context<Router>,
) -> impl gpui::IntoElement {
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
                .bg(rgb(palette.surface))
                .border_1()
                .border_color(rgb(palette.yellow))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(palette.yellow))
                .child("!"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(palette.text))
                .child("Couldn't load notifications"),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.subtext))
                .text_center()
                .child(message),
        )
        .child(
            div()
                .h(px(44.))
                .px_6()
                .rounded_xl()
                .bg(rgb(palette.text))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(rgb(palette.bg))
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
    unread_only: bool,
) -> Vec<&'a Notification> {
    let q = query.to_ascii_lowercase();
    notifications
        .iter()
        .filter(|n| !unread_only || n.unread)
        .filter(|n| {
            n.subject.title.to_ascii_lowercase().contains(&q)
                || n.repository.full_name.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

fn notif_type_style(palette: Palette, kind: &str) -> (u32, &'static str) {
    match kind {
        "PullRequest" => (palette.purple, "PR"),
        "Issue" => (palette.green, "#"),
        "Release" => (palette.accent, "R"),
        "CheckSuite" => (palette.yellow, "CI"),
        _ => (palette.subtext, "N"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{NotifRepo, NotifSubject};

    fn notification(id: &str, unread: bool, title: &str, repo: &str) -> Notification {
        Notification {
            id: id.into(),
            unread,
            reason: "mention".into(),
            updated_at: String::new(),
            repository: NotifRepo {
                full_name: repo.into(),
                html_url: String::new(),
            },
            subject: NotifSubject {
                title: title.into(),
                kind: "Issue".into(),
                url: None,
                latest_comment_url: None,
            },
        }
    }

    #[test]
    fn unread_filter_combines_with_title_and_repository_search() {
        let items = vec![
            notification("1", true, "Fix layout", "sample/mobile"),
            notification("2", false, "Fix layout", "sample/mobile"),
            notification("3", true, "Update docs", "sample/docs"),
        ];
        for (query, unread, expected) in [
            ("", false, vec!["1", "2", "3"]),
            ("", true, vec!["1", "3"]),
            ("LAYOUT", true, vec!["1"]),
            ("MOBILE", false, vec!["1", "2"]),
            ("docs", true, vec!["3"]),
            ("missing", true, vec![]),
        ] {
            let actual: Vec<_> = filter_notifications(&items, query, unread)
                .iter()
                .map(|n| n.id.as_str())
                .collect();
            assert_eq!(actual, expected, "query={query}, unread={unread}");
        }
    }
}
