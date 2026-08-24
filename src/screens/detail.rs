use gpui::{div, prelude::*, px, rgb};

use super::{safe_area_insets, Router, Screen};

const BG: u32 = 0x0D1117;
const SURFACE: u32 = 0x161B22;
const BORDER: u32 = 0x30363D;
const TEXT: u32 = 0xE6EDF3;
const SUBTEXT: u32 = 0x8B949E;
const ACCENT: u32 = 0x58A6FF;
const GREEN: u32 = 0x3FB950;
const YELLOW: u32 = 0xF0883E;

pub fn render(router: &mut Router, cx: &mut gpui::Context<Router>) -> gpui::AnyElement {
    let Some(notification) = router.selected_notification.clone() else {
        router.current_screen = Screen::Home;
        return div().size_full().bg(rgb(BG)).into_any_element();
    };

    let detail = router.notification_detail.clone();
    let loading = router.notification_detail_loading;
    let error = router.notification_detail_error.clone();
    let (safe_top, safe_bottom, _, _) = safe_area_insets();
    let (type_color, type_label) = type_style(&notification.subject.kind);

    div()
        .size_full()
        .bg(rgb(BG))
        .pt(px(safe_top))
        .pb(px(safe_bottom))
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(60.))
                .px_4()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .size(px(40.))
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_lg()
                        .text_color(rgb(TEXT))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|router, _, _, cx| router.close_notification(cx)),
                        )
                        .child("‹"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_base()
                                .text_color(rgb(TEXT))
                                .child("Notification"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(SUBTEXT))
                                .truncate()
                                .child(notification.repository.full_name.clone()),
                        ),
                ),
        )
        .child(
            div()
                .id("notification-detail-scroll")
                .flex_1()
                .overflow_y_scroll()
                .p_5()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .px_3()
                                .py_1()
                                .rounded_full()
                                .bg(gpui::rgba(type_color * 256 + 0x26))
                                .text_xs()
                                .text_color(rgb(type_color))
                                .child(type_label),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(if notification.unread {
                                    rgb(ACCENT)
                                } else {
                                    rgb(SUBTEXT)
                                })
                                .child(if notification.unread {
                                    "Unread"
                                } else {
                                    "Read"
                                }),
                        ),
                )
                .child(
                    div()
                        .text_xl()
                        .text_color(rgb(TEXT))
                        .child(notification.subject.title.clone()),
                )
                .child(
                    div()
                        .p_4()
                        .rounded_xl()
                        .bg(rgb(SURFACE))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(meta_row("Reason", reason_label(&notification.reason)))
                        .child(meta_row(
                            "Updated",
                            format_timestamp(&notification.updated_at),
                        ))
                        .children(detail.as_ref().and_then(|detail| {
                            detail
                                .author
                                .as_ref()
                                .map(|author| meta_row("Author", author))
                        }))
                        .children(detail.as_ref().and_then(|detail| {
                            detail.state.as_ref().map(|state| meta_row("State", state))
                        }))
                        .children(detail.as_ref().and_then(|detail| {
                            detail
                                .comments
                                .map(|comments| meta_row("Comments", comments.to_string()))
                        }))
                        .children(detail.as_ref().and_then(|detail| {
                            detail
                                .created_at
                                .as_ref()
                                .map(|created| meta_row("Created", format_timestamp(created)))
                        }))
                        .children(detail.as_ref().and_then(|detail| {
                            detail.updated_at.as_ref().and_then(|updated| {
                                (updated != &notification.updated_at)
                                    .then(|| meta_row("Detail updated", format_timestamp(updated)))
                            })
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(SUBTEXT))
                                .child("Description"),
                        )
                        .child(
                            div()
                                .p_4()
                                .rounded_xl()
                                .bg(rgb(SURFACE))
                                .border_1()
                                .border_color(if error.is_some() {
                                    rgb(YELLOW)
                                } else {
                                    rgb(BORDER)
                                })
                                .text_sm()
                                .text_color(rgb(TEXT))
                                .child(if loading {
                                    "Loading details…".to_string()
                                } else if let Some(message) = error {
                                    message
                                } else {
                                    detail.and_then(|detail| detail.body).unwrap_or_else(|| {
                                        "No description was provided.".to_string()
                                    })
                                }),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .h(px(48.))
                        .min_h(px(48.))
                        .w_full()
                        .rounded_xl()
                        .bg(rgb(ACCENT))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(rgb(BG))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|router, _, _, _cx| {
                                router.open_selected_notification_in_github();
                            }),
                        )
                        .child("Open on GitHub"),
                ),
        )
        .into_any_element()
}

fn meta_row(label: impl Into<String>, value: impl Into<String>) -> gpui::AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .justify_between()
        .gap_4()
        .child(div().text_xs().text_color(rgb(SUBTEXT)).child(label.into()))
        .child(
            div()
                .flex_1()
                .text_right()
                .text_xs()
                .text_color(rgb(TEXT))
                .child(value.into()),
        )
        .into_any_element()
}

fn type_style(kind: &str) -> (u32, &'static str) {
    match kind {
        "PullRequest" => (0xBC8CFF, "Pull request"),
        "Issue" => (GREEN, "Issue"),
        "Release" => (ACCENT, "Release"),
        "CheckSuite" => (YELLOW, "Checks"),
        _ => (SUBTEXT, "Notification"),
    }
}

fn reason_label(reason: &str) -> &'static str {
    match reason {
        "assign" => "Assigned to you",
        "author" => "You authored this",
        "comment" => "New comment",
        "mention" => "You were mentioned",
        "review_requested" => "Review requested",
        "subscribed" => "Subscribed",
        "team_mention" => "Team mention",
        "ci_activity" => "CI activity",
        _ => "Repository activity",
    }
}

fn format_timestamp(timestamp: &str) -> String {
    timestamp
        .replace('T', " ")
        .trim_end_matches('Z')
        .to_string()
}
