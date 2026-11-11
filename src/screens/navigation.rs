use super::{theme::Palette, Router, Screen};
use gpui::{div, prelude::*, px, rgb};

pub fn render(
    palette: Palette,
    selected: Screen,
    cx: &mut gpui::Context<Router>,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .px_4()
        .py_2()
        .gap_3()
        .border_t_1()
        .border_color(rgb(palette.border))
        .children(
            [
                (Screen::Home, "Inbox"),
                (Screen::Repositories, "Repositories"),
            ]
            .into_iter()
            .map(|(screen, label)| {
                let active = selected == screen;
                div()
                    .id(label)
                    .flex_1()
                    .min_w_0()
                    .h(px(52.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_b_2()
                    .border_color(rgb(if active { palette.accent } else { palette.bg }))
                    .text_sm()
                    .text_color(rgb(if active {
                        palette.accent
                    } else {
                        palette.subtext
                    }))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |router, _, _, cx| {
                            if router.current_screen == screen {
                                return;
                            }
                            if screen == Screen::Repositories {
                                router.open_repositories(cx);
                            } else {
                                router.go_back(cx);
                            }
                        }),
                    )
                    .child(label)
            }),
        )
}
