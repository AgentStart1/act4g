use super::theme::Palette;
use super::{safe_area_insets, Router, Screen};
use crate::api::ItemFilter;
use gpui::{div, prelude::*, px, rgb, SharedString};

pub fn render(router: &mut Router, cx: &mut gpui::Context<Router>) -> impl IntoElement {
    let palette = router.palette;
    let detail = router.current_screen == Screen::RepositoryItems;
    let loading = if detail {
        router.repository_items_loading
    } else {
        router.repositories_loading
    };
    let error = if detail {
        &router.repository_items_error
    } else {
        &router.repositories_error
    };
    let title = if detail {
        router
            .selected_repository
            .as_ref()
            .map(|repo| repo.full_name.clone())
            .unwrap_or_default()
    } else {
        "Repositories".to_string()
    };
    let scroll = if detail {
        router.scroll_positions.items(&title, router.item_filter)
    } else {
        router.scroll_positions.repositories.clone()
    };
    let (top, bottom, left, right) = safe_area_insets();
    let mut content = div()
        .id(SharedString::from(if detail {
            format!("repository-items-scroll-{}-{:?}", title, router.item_filter)
        } else {
            "repositories-scroll".to_string()
        }))
        .flex_1()
        .min_w_0()
        .min_h_0()
        .overflow_y_scroll()
        .when(
            !detail || !loading || !router.repository_items.is_empty(),
            |view| view.track_scroll(&scroll),
        )
        .flex()
        .flex_col()
        .gap_3()
        .p_4();
    if loading {
        content = content.child(div().text_sm().text_color(rgb(palette.subtext)).child(
            if detail {
                "Loading open issues and pull requests…"
            } else {
                "Loading repositories and counts…"
            },
        ));
    }
    if let Some(error) = error {
        content = content.child(
            div()
                .p_3()
                .rounded_xl()
                .bg(rgb(palette.surface))
                .text_sm()
                .text_color(rgb(palette.yellow))
                .child(error.clone()),
        );
    }
    if detail {
        let mut issues = router
            .repository_items
            .iter()
            .filter(|item| item.pull_request.is_none())
            .count();
        let mut pulls = router.repository_items.len() - issues;
        if loading && router.repository_items.is_empty() {
            if let Some(summary) = router.repositories.iter().find(|summary| {
                router
                    .selected_repository
                    .as_ref()
                    .is_some_and(|repo| repo.full_name == summary.repository.full_name)
            }) {
                issues = summary.issues as usize;
                pulls = summary.pull_requests as usize;
            }
        }
        content = content.child(
            div()
                .flex()
                .gap_3()
                .child(filter_button(
                    palette,
                    format!("Issues ({issues})"),
                    ItemFilter::Issues,
                    router.item_filter,
                    cx,
                ))
                .child(filter_button(
                    palette,
                    format!("PRs ({pulls})"),
                    ItemFilter::PullRequests,
                    router.item_filter,
                    cx,
                )),
        );
        let items: Vec<_> = router
            .repository_items
            .iter()
            .filter(|item| {
                item.pull_request.is_some() == (router.item_filter == ItemFilter::PullRequests)
            })
            .collect();
        if items.is_empty() && !loading && error.is_none() {
            content = content.child(
                div()
                    .py_8()
                    .text_sm()
                    .text_color(rgb(palette.subtext))
                    .child(if router.item_filter == ItemFilter::Issues {
                        "No open issues"
                    } else {
                        "No open pull requests"
                    }),
            );
        }
        for item in items {
            let url = item.html_url.clone();
            let author = item
                .user
                .as_ref()
                .map(|user| format!("@{}", user.login))
                .unwrap_or_else(|| "Unknown author".into());
            content = content.child(
                div()
                    .id(SharedString::from(format!(
                        "repository-item-{}",
                        item.number
                    )))
                    .min_w_0()
                    .flex_shrink_0()
                    .p_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |_, _, _, _| {
                            if let Err(error) =
                                gpui_mobile::packages::url_launcher::launch_url(&url)
                            {
                                log::warn!("Failed to open repository item: {error}");
                            }
                        }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .line_clamp(2)
                            .text_ellipsis()
                            .text_base()
                            .text_color(rgb(palette.text))
                            .child(item.title.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.subtext))
                            .child(format!("#{} · {}", item.number, author)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.accent))
                            .child("Open on GitHub  →"),
                    ),
            );
        }
    } else {
        content = content.child(div().text_xs().text_color(rgb(palette.subtext)).child(
            "Personal, organization and collaborator repositories · Empty repositories hidden",
        ));
        if router.repositories.is_empty() && !loading && error.is_none() {
            content = content.child(
                div()
                    .py_8()
                    .text_sm()
                    .text_color(rgb(palette.subtext))
                    .child("No repositories with open issues or pull requests"),
            );
        }
        for summary in &router.repositories {
            let repository = summary.repository.clone();
            content = content.child(
                div()
                    .id(SharedString::from(format!(
                        "repository-{}",
                        repository.full_name
                    )))
                    .min_w_0()
                    .flex_shrink_0()
                    .p_4()
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |router, _, _, cx| {
                            router.open_repository(repository.clone(), cx)
                        }),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .line_clamp(2)
                            .text_ellipsis()
                            .text_base()
                            .text_color(rgb(palette.text))
                            .child(summary.repository.full_name.clone()),
                    )
                    .child(div().text_xs().text_color(rgb(palette.subtext)).child(
                        if summary.repository.private {
                            "Private"
                        } else {
                            "Public"
                        },
                    ))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(palette.accent))
                            .child(format!(
                                "{} Issues · {} PRs  →",
                                summary.issues, summary.pull_requests
                            )),
                    ),
            );
        }
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .pt(px(top))
        .pb(px(bottom))
        .pl(px(left))
        .pr(px(right))
        .child(
            div()
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .min_h(px(48.))
                                .flex()
                                .items_center()
                                .text_sm()
                                .text_color(rgb(palette.accent))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |router, _, _, cx| {
                                        router.go_back(cx);
                                    }),
                                )
                                .child(if detail {
                                    "← Repositories"
                                } else {
                                    "← Inbox"
                                }),
                        )
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .min_h(px(48.))
                                .flex()
                                .items_center()
                                .text_sm()
                                .text_color(rgb(palette.accent))
                                .on_mouse_down(
                                    gpui::MouseButton::Left,
                                    cx.listener(move |router, _, _, cx| {
                                        if detail {
                                            router.refresh_repository_items(cx);
                                        } else {
                                            router.refresh_repositories(cx);
                                        }
                                    }),
                                )
                                .child(if loading {
                                    "Refreshing…"
                                } else if error.is_some() {
                                    "Retry"
                                } else {
                                    "Refresh"
                                }),
                        ),
                )
                .child(
                    div()
                        .min_w_0()
                        .line_clamp(2)
                        .text_ellipsis()
                        .text_2xl()
                        .text_color(rgb(palette.text))
                        .child(title),
                ),
        )
        .child(content)
        .when(!detail, |view| {
            view.child(super::navigation::render(palette, Screen::Repositories, cx))
        })
}

fn filter_button(
    palette: Palette,
    label: String,
    filter: ItemFilter,
    selected: ItemFilter,
    cx: &mut gpui::Context<Router>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_xl()
        .min_h(px(48.))
        .flex()
        .items_center()
        .bg(rgb(if filter == selected {
            palette.accent
        } else {
            palette.surface
        }))
        .text_sm()
        .text_color(rgb(if filter == selected {
            palette.on_accent
        } else {
            palette.subtext
        }))
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |router, _, _, cx| {
                router.item_filter = filter;
                cx.notify();
            }),
        )
        .child(label)
}
