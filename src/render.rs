//! Markdown → PDF rendering.
//!
//! The pipeline is: pulldown-cmark events → a small block/inline IR → genpdf
//! elements. Fonts (DejaVu Sans + Sans Mono) are embedded into the binary so
//! the tool has no runtime dependencies.

use std::iter::Peekable;
use std::path::Path;

use genpdf::elements;
use genpdf::fonts;
use genpdf::style::{Color, Style};
use genpdf::Element as _;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};

// ---------------------------------------------------------------------------
// Embedded fonts
// ---------------------------------------------------------------------------

const SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
const SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf");
const SANS_ITALIC: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-Oblique.ttf");
const SANS_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/DejaVuSans-BoldOblique.ttf");

const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");
const MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono-Bold.ttf");
const MONO_ITALIC: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono-Oblique.ttf");
const MONO_BOLD_ITALIC: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono-BoldOblique.ttf");

fn font_family(
    regular: &[u8],
    bold: &[u8],
    italic: &[u8],
    bold_italic: &[u8],
) -> Result<fonts::FontFamily<fonts::FontData>, genpdf::error::Error> {
    Ok(fonts::FontFamily {
        regular: fonts::FontData::new(regular.to_vec(), None)?,
        bold: fonts::FontData::new(bold.to_vec(), None)?,
        italic: fonts::FontData::new(italic.to_vec(), None)?,
        bold_italic: fonts::FontData::new(bold_italic.to_vec(), None)?,
    })
}

// ---------------------------------------------------------------------------
// Public options
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PaperSize {
    A4,
    Letter,
    Legal,
}

impl PaperSize {
    /// All supported sizes, for UI enumeration.
    pub const ALL: [PaperSize; 3] = [PaperSize::A4, PaperSize::Letter, PaperSize::Legal];

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::Letter => "Letter",
            PaperSize::Legal => "Legal",
        }
    }

    /// Page size in millimetres (width, height).
    fn dimensions(self) -> (f64, f64) {
        match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::Legal => (215.9, 355.6),
        }
    }
}

pub struct RenderOptions {
    pub title: String,
    pub paper: PaperSize,
    pub margin_mm: f64,
    pub base_font_size: u8,
}

// ---------------------------------------------------------------------------
// Intermediate representation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct Flags {
    bold: bool,
    italic: bool,
    code: bool,
    link: bool,
}

#[derive(Clone)]
struct Span {
    text: String,
    flags: Flags,
}

enum Node {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    CodeBlock(String),
    BlockQuote(Vec<Node>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Node>>,
    },
    Rule,
    Table {
        header: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
}

// ---------------------------------------------------------------------------
// Parsing: events → IR
// ---------------------------------------------------------------------------

fn parse_markdown(markdown: &str) -> Vec<Node> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let mut iter = parser.peekable();
    parse_nodes(&mut iter)
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Parse a sequence of block-level nodes. Stops (without consuming) when the
/// next event is an `End`, leaving it for the caller to consume.
fn parse_nodes<'a, I>(iter: &mut Peekable<I>) -> Vec<Node>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut nodes = Vec::new();

    while let Some(event) = iter.peek() {
        match event {
            Event::End(_) => break,
            Event::Rule => {
                iter.next();
                nodes.push(Node::Rule);
            }
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    iter.next();
                    let spans = parse_inline(iter, Flags::default());
                    consume_end(iter);
                    if !spans.is_empty() {
                        nodes.push(Node::Paragraph(spans));
                    }
                }
                Tag::Heading(level, _, _) => {
                    let lvl = heading_level(*level);
                    iter.next();
                    let spans = parse_inline(iter, Flags::default());
                    consume_end(iter);
                    nodes.push(Node::Heading(lvl, spans));
                }
                Tag::BlockQuote => {
                    iter.next();
                    let inner = parse_nodes(iter);
                    consume_end(iter);
                    nodes.push(Node::BlockQuote(inner));
                }
                Tag::CodeBlock(_) => {
                    iter.next();
                    let text = collect_code_text(iter);
                    consume_end(iter);
                    nodes.push(Node::CodeBlock(text));
                }
                Tag::List(start) => {
                    let ordered = start.is_some();
                    let start_num = start.unwrap_or(1);
                    iter.next();
                    let items = parse_list_items(iter);
                    consume_end(iter);
                    nodes.push(Node::List {
                        ordered,
                        start: start_num,
                        items,
                    });
                }
                Tag::Table(_) => {
                    iter.next();
                    let (header, rows) = parse_table(iter);
                    consume_end(iter);
                    nodes.push(Node::Table { header, rows });
                }
                _ => {
                    // An inline start (link, emphasis, image, …) appearing at
                    // block level — e.g. a tight list item whose only content
                    // is a link. Do NOT consume it here; let `parse_inline`
                    // handle the tag and its nesting, building an implicit
                    // paragraph.
                    let spans = parse_inline(iter, Flags::default());
                    if !spans.is_empty() {
                        nodes.push(Node::Paragraph(spans));
                    }
                }
            },
            // Stray inline content (e.g. tight list items) → implicit paragraph.
            Event::Text(_)
            | Event::Code(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::TaskListMarker(_) => {
                let spans = parse_inline(iter, Flags::default());
                if !spans.is_empty() {
                    nodes.push(Node::Paragraph(spans));
                }
            }
            _ => {
                iter.next();
            }
        }
    }

    nodes
}

fn parse_list_items<'a, I>(iter: &mut Peekable<I>) -> Vec<Vec<Node>>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut items = Vec::new();
    while let Some(Event::Start(Tag::Item)) = iter.peek() {
        iter.next(); // consume Start(Item)
        let item = parse_nodes(iter);
        consume_end(iter); // consume End(Item)
        items.push(item);
    }
    items
}

fn parse_table<'a, I>(iter: &mut Peekable<I>) -> (Vec<Vec<Span>>, Vec<Vec<Vec<Span>>>)
where
    I: Iterator<Item = Event<'a>>,
{
    let mut header = Vec::new();
    let mut rows = Vec::new();

    while let Some(event) = iter.peek() {
        match event {
            Event::Start(Tag::TableHead) => {
                iter.next();
                header = parse_table_row(iter);
                consume_end(iter);
            }
            Event::Start(Tag::TableRow) => {
                iter.next();
                rows.push(parse_table_row(iter));
                consume_end(iter);
            }
            Event::End(_) => break,
            _ => {
                iter.next();
            }
        }
    }

    (header, rows)
}

fn parse_table_row<'a, I>(iter: &mut Peekable<I>) -> Vec<Vec<Span>>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut cells = Vec::new();
    while let Some(Event::Start(Tag::TableCell)) = iter.peek() {
        iter.next();
        let spans = parse_inline(iter, Flags::default());
        consume_end(iter);
        cells.push(spans);
    }
    cells
}

/// Collect raw text inside a code block (no inline styling applies).
fn collect_code_text<'a, I>(iter: &mut Peekable<I>) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut text = String::new();
    while let Some(event) = iter.peek() {
        match event {
            Event::End(_) => break,
            Event::Text(t) => {
                text.push_str(t);
                iter.next();
            }
            _ => {
                iter.next();
            }
        }
    }
    while text.ends_with('\n') {
        text.pop();
    }
    text
}

/// Parse inline content into styled spans. Stops (without consuming) at the
/// first `End`, block-level `Start`, or `Rule`.
fn parse_inline<'a, I>(iter: &mut Peekable<I>, flags: Flags) -> Vec<Span>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();

    while let Some(event) = iter.peek() {
        match event {
            Event::End(_) | Event::Rule => break,
            Event::Start(tag) => match tag {
                Tag::Emphasis => {
                    iter.next();
                    let mut f = flags;
                    f.italic = true;
                    spans.extend(parse_inline(iter, f));
                    consume_end(iter);
                }
                Tag::Strong => {
                    iter.next();
                    let mut f = flags;
                    f.bold = true;
                    spans.extend(parse_inline(iter, f));
                    consume_end(iter);
                }
                Tag::Strikethrough => {
                    iter.next();
                    spans.extend(parse_inline(iter, flags));
                    consume_end(iter);
                }
                Tag::Link(_, dest, _) => {
                    let dest = dest.to_string();
                    iter.next();
                    let mut f = flags;
                    f.link = true;
                    let inner = parse_inline(iter, f);
                    consume_end(iter);
                    let label: String = inner.iter().map(|s| s.text.as_str()).collect();
                    spans.extend(inner);
                    // Append the URL unless the label already is the URL.
                    if !dest.is_empty() && label.trim() != dest {
                        spans.push(Span {
                            text: format!(" ({dest})"),
                            flags: Flags {
                                link: true,
                                ..flags
                            },
                        });
                    }
                }
                Tag::Image(_, _, _) => {
                    iter.next();
                    let alt = parse_inline(iter, flags);
                    consume_end(iter);
                    let alt_text: String = alt.iter().map(|s| s.text.as_str()).collect();
                    spans.push(Span {
                        text: format!("[image: {}]", alt_text.trim()),
                        flags: Flags {
                            italic: true,
                            ..flags
                        },
                    });
                }
                // A block-level start ends the inline run.
                _ => break,
            },
            Event::Text(t) => {
                spans.push(Span {
                    text: t.to_string(),
                    flags,
                });
                iter.next();
            }
            Event::Code(t) => {
                spans.push(Span {
                    text: t.to_string(),
                    flags: Flags {
                        code: true,
                        ..flags
                    },
                });
                iter.next();
            }
            Event::SoftBreak => {
                spans.push(Span {
                    text: " ".to_string(),
                    flags,
                });
                iter.next();
            }
            Event::HardBreak => {
                spans.push(Span {
                    text: " ".to_string(),
                    flags,
                });
                iter.next();
            }
            Event::TaskListMarker(checked) => {
                spans.push(Span {
                    text: if *checked { "[x] " } else { "[ ] " }.to_string(),
                    flags,
                });
                iter.next();
            }
            _ => {
                iter.next();
            }
        }
    }

    spans
}

fn consume_end<'a, I>(iter: &mut Peekable<I>)
where
    I: Iterator<Item = Event<'a>>,
{
    if let Some(Event::End(_)) = iter.peek() {
        iter.next();
    }
}

// ---------------------------------------------------------------------------
// Rendering: IR → genpdf
// ---------------------------------------------------------------------------

struct Theme {
    mono: fonts::FontFamily<fonts::Font>,
    base_size: u8,
    link: Color,
    code: Color,
    heading: Color,
    rule: Color,
}

fn span_style(span: &Span, theme: &Theme) -> Style {
    let mut style = Style::new();
    if span.flags.code {
        style.set_font_family(theme.mono);
        style.set_color(theme.code);
    }
    if span.flags.bold {
        style.set_bold();
    }
    if span.flags.italic {
        style.set_italic();
    }
    if span.flags.link {
        style.set_color(theme.link);
    }
    style
}

fn heading_size(level: u8, base: u8) -> u8 {
    let base = base as u16;
    let size = match level {
        1 => base * 2,
        2 => base + base / 2 + 2,
        3 => base + base / 3 + 2,
        4 => base + 2,
        5 => base + 1,
        _ => base,
    };
    size.min(64) as u8
}

fn build_paragraph(spans: &[Span], base_style: Style, theme: &Theme) -> elements::Paragraph {
    let mut para = elements::Paragraph::default();
    for span in spans {
        let mut style = base_style;
        let s = span_style(span, theme);
        style = style.and(s);
        para.push_styled(span.text.clone(), style);
    }
    para
}

fn render_nodes(nodes: &[Node], theme: &Theme) -> elements::LinearLayout {
    let mut layout = elements::LinearLayout::vertical();

    for node in nodes {
        match node {
            Node::Heading(level, spans) => {
                let size = heading_size(*level, theme.base_size);
                let style = Style::new()
                    .bold()
                    .with_font_size(size)
                    .with_color(theme.heading);
                layout.push(elements::Break::new(0.5));
                layout.push(build_paragraph(spans, style, theme));
                layout.push(elements::Break::new(0.3));
            }
            Node::Paragraph(spans) => {
                let style = Style::new().with_font_size(theme.base_size);
                layout.push(build_paragraph(spans, style, theme));
                layout.push(elements::Break::new(0.5));
            }
            Node::CodeBlock(text) => {
                let mut inner = elements::LinearLayout::vertical();
                let style = Style::new()
                    .with_font_family(theme.mono)
                    .with_font_size(theme.base_size.saturating_sub(1).max(7))
                    .with_color(theme.code);
                if text.is_empty() {
                    inner.push(elements::Paragraph::new(" "));
                } else {
                    for line in text.split('\n') {
                        let display = if line.is_empty() { " " } else { line };
                        inner.push(
                            elements::Paragraph::default()
                                .styled_string(display.to_string(), style),
                        );
                    }
                }
                let framed = inner.framed().padded(genpdf::Margins::trbl(2, 3, 2, 3));
                layout.push(framed);
                layout.push(elements::Break::new(0.5));
            }
            Node::BlockQuote(inner) => {
                let inner_layout = render_nodes(inner, theme);
                layout.push(inner_layout.padded(genpdf::Margins::trbl(0, 0, 0, 8)));
                layout.push(elements::Break::new(0.5));
            }
            Node::List {
                ordered,
                start,
                items,
            } => {
                if *ordered {
                    let mut list = elements::OrderedList::with_start(*start as usize);
                    for item in items {
                        list.push(render_nodes(item, theme));
                    }
                    layout.push(list);
                } else {
                    let mut list = elements::UnorderedList::new();
                    for item in items {
                        list.push(render_nodes(item, theme));
                    }
                    layout.push(list);
                }
                layout.push(elements::Break::new(0.5));
            }
            Node::Rule => {
                layout.push(elements::Break::new(0.3));
                let line = "_".repeat(80);
                layout.push(
                    elements::Paragraph::default()
                        .styled_string(line, Style::new().with_color(theme.rule).with_font_size(6)),
                );
                layout.push(elements::Break::new(0.3));
            }
            Node::Table { header, rows } => {
                if let Some(table) = build_table(header, rows, theme) {
                    layout.push(table);
                    layout.push(elements::Break::new(0.5));
                }
            }
        }
    }

    layout
}

fn build_table(
    header: &[Vec<Span>],
    rows: &[Vec<Vec<Span>>],
    theme: &Theme,
) -> Option<elements::TableLayout> {
    let cols = header
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    if cols == 0 {
        return None;
    }

    let mut table = elements::TableLayout::new(vec![1; cols]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, true));

    let header_style = Style::new().bold().with_font_size(theme.base_size);
    let cell_style = Style::new().with_font_size(theme.base_size);

    let push_row = |table: &mut elements::TableLayout, cells: &[Vec<Span>], style: Style| {
        let mut row = table.row();
        for c in 0..cols {
            let empty = Vec::new();
            let spans = cells.get(c).unwrap_or(&empty);
            let para =
                build_paragraph(spans, style, theme).padded(genpdf::Margins::trbl(1, 2, 1, 2));
            row.push_element(para);
        }
        let _ = row.push();
    };

    if !header.is_empty() {
        push_row(&mut table, header, header_style);
    }
    for r in rows {
        push_row(&mut table, r, cell_style);
    }

    Some(table)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn markdown_to_pdf(
    markdown: &str,
    output: &Path,
    opts: &RenderOptions,
) -> Result<(), genpdf::error::Error> {
    let sans = font_family(SANS_REGULAR, SANS_BOLD, SANS_ITALIC, SANS_BOLD_ITALIC)?;
    let mono_data = font_family(MONO_REGULAR, MONO_BOLD, MONO_ITALIC, MONO_BOLD_ITALIC)?;

    let mut doc = genpdf::Document::new(sans);
    doc.set_title(opts.title.clone());
    doc.set_font_size(opts.base_font_size);
    doc.set_line_spacing(1.35);

    let (w, h) = opts.paper.dimensions();
    doc.set_paper_size(genpdf::Size::new(w, h));

    let mono = doc.add_font_family(mono_data);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(opts.margin_mm as f32);
    doc.set_page_decorator(decorator);

    let theme = Theme {
        mono,
        base_size: opts.base_font_size,
        link: Color::Rgb(0x1a, 0x5f, 0xb4),
        code: Color::Rgb(0xb0, 0x30, 0x60),
        heading: Color::Rgb(0x11, 0x11, 0x11),
        rule: Color::Rgb(0xaa, 0xaa, 0xaa),
    };

    let nodes = parse_markdown(markdown);
    let content = render_nodes(&nodes, &theme);
    doc.push(content);

    doc.render_to_file(output)?;
    Ok(())
}
