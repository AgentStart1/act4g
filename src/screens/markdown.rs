use super::theme::Palette;
use gpui::{div, prelude::*, px, rgb, FontStyle, FontWeight, HighlightStyle, StyledText};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Clone, Default)]
pub struct Document {
    blocks: Vec<Block>,
}

#[derive(Clone, Default)]
struct Block {
    text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    links: Vec<(Range<usize>, String)>,
    heading: bool,
    code: bool,
    quote: bool,
    indent: usize,
    rule: bool,
    cells: Vec<Block>,
    image: Option<String>,
}

impl Document {
    pub fn parse(source: &str) -> Self {
        let mut document = Self::default();
        let mut block = Block::default();
        let mut styles = Vec::new();
        let mut links = Vec::new();
        let mut lists: Vec<Option<u64>> = Vec::new();
        let mut quotes = 0;
        let mut cells = Vec::new();
        let mut table_heading = false;
        let options =
            Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
        for event in Parser::new_ext(source, options) {
            match event {
                Event::Start(Tag::Paragraph) => {}
                Event::Start(Tag::Heading { .. }) => block.heading = true,
                Event::Start(Tag::CodeBlock(_)) => block.code = true,
                Event::Start(Tag::BlockQuote(_)) => {
                    flush(&mut document, &mut block);
                    quotes += 1;
                }
                Event::Start(Tag::List(start)) => {
                    flush(&mut document, &mut block);
                    lists.push(start);
                }
                Event::Start(Tag::Item) => {
                    flush(&mut document, &mut block);
                    block.indent = lists.len().saturating_sub(1);
                    if let Some(Some(number)) = lists.last_mut() {
                        block.text.push_str(&format!("{number}. "));
                        *number += 1;
                    } else {
                        block.text.push_str("• ");
                    }
                }
                Event::Start(Tag::Emphasis) => styles.push(HighlightStyle {
                    font_style: Some(FontStyle::Italic),
                    ..Default::default()
                }),
                Event::Start(Tag::Strong) => styles.push(HighlightStyle {
                    font_weight: Some(FontWeight::BOLD),
                    ..Default::default()
                }),
                Event::Start(Tag::Strikethrough) => styles.push(HighlightStyle {
                    strikethrough: Some(Default::default()),
                    ..Default::default()
                }),
                Event::Start(Tag::Image { dest_url, .. }) => {
                    flush(&mut document, &mut block);
                    block.image = Some(dest_url.to_string());
                }
                Event::End(TagEnd::Image) => flush(&mut document, &mut block),
                Event::Start(Tag::Link { dest_url, .. }) => {
                    links.push((block.text.len(), dest_url.to_string()));
                    styles.push(HighlightStyle {
                        color: Some(rgb(0x58A6FF).into()),
                        ..Default::default()
                    });
                }
                Event::Start(Tag::TableHead) => table_heading = true,
                Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {
                    styles.pop();
                }
                Event::End(TagEnd::Link) => {
                    if let Some((start, url)) = links.pop() {
                        if block.text.len() == start {
                            block.text.push_str(&url);
                        }
                        block.links.push((start..block.text.len(), url));
                    }
                    styles.pop();
                }
                Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                    block.quote = quotes > 0;
                    block.indent = block.indent.max(lists.len().saturating_sub(1));
                    append(&mut block, &text, &styles);
                }
                Event::Code(text) => {
                    let mut code_styles = styles.clone();
                    code_styles.push(HighlightStyle {
                        background_color: Some(rgb(0x30363D).into()),
                        ..Default::default()
                    });
                    append(&mut block, &text, &code_styles);
                }
                Event::SoftBreak => append(&mut block, " ", &styles),
                Event::HardBreak => append(&mut block, "\n", &styles),
                Event::TaskListMarker(checked) => {
                    append(&mut block, if checked { "☑ " } else { "☐ " }, &styles)
                }
                Event::Rule => {
                    flush(&mut document, &mut block);
                    block.rule = true;
                    flush(&mut document, &mut block);
                }
                Event::End(TagEnd::TableCell) => {
                    block.heading = table_heading;
                    cells.push(std::mem::take(&mut block));
                }
                Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                    document.blocks.push(Block {
                        cells: std::mem::take(&mut cells),
                        ..Default::default()
                    });
                    table_heading = false;
                }
                Event::End(
                    TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock | TagEnd::Item,
                ) => flush(&mut document, &mut block),
                Event::End(TagEnd::List(_)) => {
                    flush(&mut document, &mut block);
                    lists.pop();
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    flush(&mut document, &mut block);
                    quotes -= 1;
                }
                _ => {}
            }
        }
        flush(&mut document, &mut block);
        document
    }

    pub fn render(&self, palette: Palette, cx: &gpui::App) -> gpui::AnyElement {
        // The mobile text backend resolves real family names, not CSS generic aliases.
        let fonts = cx.text_system().all_font_names();
        let code_font = [
            "Droid Sans Mono",
            "Roboto Mono",
            "Menlo",
            "Consolas",
            "DejaVu Sans Mono",
            "monospace",
        ]
        .into_iter()
        .find(|candidate| fonts.iter().any(|name| name == candidate));
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .children(self.blocks.iter().enumerate().map(|(index, block)| {
                if let Some(url) = &block.image {
                    let url = url.clone();
                    return div()
                        .w_full()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .when(
                            url.starts_with("https://") || url.starts_with("http://"),
                            |el| {
                                el.child(
                                    gpui::img(gpui::SharedString::from(url.clone()))
                                        .w_full()
                                        .max_h(px(400.))
                                        .object_fit(gpui::ObjectFit::Contain),
                                )
                            },
                        )
                        .child(
                            div()
                                .text_color(rgb(palette.accent))
                                .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                                    open_link(&url)
                                })
                                .child(if block.text.is_empty() {
                                    "Open image".to_string()
                                } else {
                                    block.text.clone()
                                }),
                        )
                        .into_any_element();
                }
                if !block.cells.is_empty() {
                    return div()
                        .w_full()
                        .flex()
                        .children(block.cells.iter().enumerate().map(|(cell_index, cell)| {
                            div()
                                .flex_1()
                                .min_w_0()
                                .p_2()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .when(cell.heading, |el| el.font_weight(FontWeight::BOLD))
                                .child(inline_text(
                                    palette,
                                    cell,
                                    format!("markdown-{index}-{cell_index}"),
                                ))
                        }))
                        .into_any_element();
                }
                div()
                    .w_full()
                    .min_w_0()
                    .flex_none()
                    .pl(px(block.indent as f32 * 16.))
                    .when(block.heading, |el| {
                        el.text_lg().font_weight(FontWeight::BOLD)
                    })
                    .when(block.code, |el| {
                        el.p_3()
                            .rounded_lg()
                            .bg(rgb(palette.bg))
                            .when_some(code_font, |el, family| el.font_family(family))
                    })
                    .when(block.quote, |el| {
                        el.pl_3().border_l_2().border_color(rgb(palette.subtext))
                    })
                    .when(block.rule, |el| el.h(px(1.)).bg(rgb(palette.border)))
                    .child(inline_text(palette, block, format!("markdown-{index}")))
                    .into_any_element()
            }))
            .into_any_element()
    }
}

// Until https://github.com/zed-industries/zed/pull/66695 reaches the pinned GPUI,
// InteractiveText installs mouse-up only after a redraw. A div's
// click handler supports same-frame touch taps; keep range checks for both ends.
fn inline_text(palette: Palette, block: &Block, id: String) -> gpui::AnyElement {
    let text =
        StyledText::new(block.text.clone()).with_highlights(themed_highlights(block, palette));
    let layout = text.layout().clone();
    let links = block.links.clone();
    div()
        .id(gpui::SharedString::from(id))
        .child(text)
        .on_click(move |event, _, _| {
            if !event.standard_click() {
                return;
            }
            let (down, up) = match event {
                gpui::ClickEvent::Mouse(event) => (event.down.position, event.up.position),
                gpui::ClickEvent::Touch(event) => (event.position, event.position),
                gpui::ClickEvent::Keyboard(_) => return,
            };
            if let (Ok(down), Ok(up)) = (
                layout.index_for_position(down),
                layout.index_for_position(up),
            ) {
                if let Some((_, url)) = links
                    .iter()
                    .find(|(range, _)| range.contains(&down) && range.contains(&up))
                {
                    open_link(url);
                }
            }
        })
        .into_any_element()
}

fn themed_highlights(
    block: &Block,
    palette: Palette,
) -> impl Iterator<Item = (Range<usize>, HighlightStyle)> + '_ {
    block.highlights.iter().map(move |(range, style)| {
        let mut style = *style;
        if style.color.is_some() {
            style.color = Some(rgb(palette.accent).into());
        }
        if style.background_color.is_some() {
            style.background_color = Some(rgb(palette.code_bg).into());
        }
        (range.clone(), style)
    })
}

fn append(block: &mut Block, text: &str, styles: &[HighlightStyle]) {
    let start = block.text.len();
    block.text.push_str(text);
    let mut style = HighlightStyle::default();
    for next in styles {
        style.font_weight = next.font_weight.or(style.font_weight);
        style.font_style = next.font_style.or(style.font_style);
        style.color = next.color.or(style.color);
        style.background_color = next.background_color.or(style.background_color);
        style.strikethrough = next.strikethrough.or(style.strikethrough);
    }
    if start != block.text.len() && !styles.is_empty() {
        block.highlights.push((start..block.text.len(), style));
    }
}

fn flush(document: &mut Document, block: &mut Block) {
    if !block.text.is_empty() || block.rule || block.image.is_some() {
        document.blocks.push(std::mem::take(block));
    }
}

fn open_link(url: &str) {
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) {
        return;
    }
    match gpui_mobile::packages::url_launcher::launch_url(url) {
        Ok(true) => {}
        Ok(false) => log::warn!("No app could open Markdown link"),
        Err(error) => log::warn!("Failed to open Markdown link: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_markdown_recolors_links_and_inline_code_when_theme_changes() {
        let doc = Document::parse("[Link](https://example.com) and `code`");
        let block = &doc.blocks[0];
        for dark in [false, true, false] {
            let palette = Palette::for_dark(dark);
            let highlights: Vec<_> = themed_highlights(block, palette).collect();
            assert!(highlights
                .iter()
                .any(|(_, style)| style.color == Some(rgb(palette.accent).into())));
            assert!(highlights
                .iter()
                .any(|(_, style)| style.background_color == Some(rgb(palette.code_bg).into())));
        }
    }

    #[test]
    fn formats_nested_inline_styles_and_unicode_links() {
        let document =
            Document::parse("**粗体 *斜体*** [链接](https://example.com) `code` ~~old~~");
        let block = &document.blocks[0];
        assert_eq!(block.text, "粗体 斜体 链接 code old");
        assert_eq!(&block.text[block.links[0].0.clone()], "链接");
        assert!(block
            .highlights
            .iter()
            .any(|(_, style)| style.font_weight.is_some() && style.font_style.is_some()));
        assert!(block
            .highlights
            .iter()
            .any(|(_, style)| style.background_color.is_some()));
        assert!(block
            .highlights
            .iter()
            .any(|(_, style)| style.strikethrough.is_some()));
    }

    #[test]
    fn parses_block_structure_and_preserves_code() {
        let document = Document::parse(
            "# Title\n\n> Quote\n\n3. first\n4. second\n\n```rust\nlet x = 1;\n```\n\n---",
        );
        assert!(document.blocks[0].heading);
        assert!(document.blocks[1].quote);
        assert_eq!(document.blocks[2].text, "3. first");
        assert_eq!(document.blocks[3].text, "4. second");
        assert!(document.blocks[4].code);
        assert_eq!(document.blocks[4].text, "let x = 1;\n");
        assert!(document.blocks[5].rule);
    }

    #[test]
    fn parses_github_task_lists() {
        let document = Document::parse("- [x] done\n- [ ] todo");
        assert!(document.blocks[0].text.contains("☑ done"));
        assert!(document.blocks[1].text.contains("☐ todo"));
    }

    #[test]
    fn preserves_table_cells_and_header_style() {
        let document = Document::parse("| A | B |\n|---|---|\n| one | two |");
        assert!(document.blocks[0].cells.iter().all(|cell| cell.heading));
        assert_eq!(document.blocks[1].cells[0].text, "one");
        assert_eq!(document.blocks[1].cells[1].text, "two");
    }

    #[test]
    fn keeps_images_and_their_alt_text_between_paragraphs() {
        let document =
            Document::parse("before\n\n![diagram](https://example.com/image.png)\n\nafter");
        assert_eq!(document.blocks[0].text, "before");
        assert_eq!(
            document.blocks[1].image.as_deref(),
            Some("https://example.com/image.png")
        );
        assert_eq!(document.blocks[1].text, "diagram");
        assert_eq!(document.blocks[2].text, "after");
    }
}
