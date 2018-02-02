use std::path::Path as FsPath;

use FormattedText;
use Song;
use Verse;

extern crate cairo;
extern crate pango;
extern crate pangocairo;

use pango::LayoutExt;

use self::pangocairo::functions as pc;
use pango::FontDescription;

type Cr = cairo::Context;
type Points = f64;

const PAGE_WIDTH: Points = 8.5 * 72.0;
const PAGE_HEIGHT: Points = 11.0 * 72.0;
const INDENT: Points = 24.0;
const FONT_SIZE: i32 = 16; // points
const MIN_FONT_SIZE: i32 = 13; // points
const MARGIN_RIGHT: Points = 0.5 * 72.0;
const GUTTER: Points = 18.0;

fn points_from_inches(size: f64) -> f64 {
    size * 72.0
}
fn points_from_pango<I: Into<f64>>(size: I) -> f64 {
    size.into() / pango::SCALE as f64
}
fn pango_from_points(size: f64) -> f64 {
    size * pango::SCALE as f64
}

#[derive(Debug)]
struct Size (f64, f64);
impl<I: Into<f64>> From<(I, I)> for Size {
    fn from(tuple: (I, I)) -> Size {
        Size (tuple.0.into(), tuple.1.into())
    }
}
impl Size {
    fn map<F: Fn(f64) -> f64>(mut self, f: F) -> Size {
        self.0 = f(self.0);
        self.1 = f(self.1);
        self
    }
    fn width(&self) -> f64 {
        self.0
    }
    fn height(&self) -> f64 {
        self.1
    }
}

struct Maximum(f64);
impl Maximum {
    fn new(init: f64) -> Maximum {
        Maximum(init)
    }
    fn see(&mut self, sample: f64) {
        self.0 = self.0.max(sample);
    }
    fn get(&self) -> f64 {
        self.0
    }
}
impl Default for Maximum {
    fn default() -> Self {
        Self::new(f64::default())
    }
}

struct FontDesc(pango::FontDescription);
unsafe impl Sync for FontDesc {}
impl<'a> From<&'a FontDesc> for Option<&'a pango::FontDescription> {
    fn from(d: &FontDesc) -> Option<&pango::FontDescription> {
        Some(&d.0)
    }
}
impl ::std::ops::Deref for FontDesc {
    type Target = pango::FontDescription;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

lazy_static! {
    static ref BASE_FONT: FontDesc = {
        let mut font = pango::FontDescription::new();
        font.set_family("Caladea");
        font.set_absolute_size(pango_from_points(12.0));
        FontDesc(font)
    };
}

pub fn pdf_song(path: &FsPath, song: &Song) -> Result<(), ()> {
    let surface = cairo::PDFSurface::create(path, PAGE_WIDTH, PAGE_HEIGHT);
    let cr = cairo::Context::new(&surface);

    cr.move_to(points_from_inches(1.5), points_from_inches(0.5));
    draw_title(&cr, song);

    try_draw_verses(&cr, song)?;

    draw_file_letter(&cr, song);
    cr.show_page();
    Ok(())
}

lazy_static! {
    static ref PAGING_FONT: FontDesc = {
        let mut font = BASE_FONT.clone();
        font.set_absolute_size(pango_from_points(12.0));
        FontDesc(font)
    };
}

fn draw_file_letter(cr: &Cr, song: &Song) {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(&*PAGING_FONT);

    let title = song.file_as().expect("Song requires a title");
    let letter = title.chars().next().expect("Song needs a non-empty title");

    layout.set_text(&letter.to_string());
    let (width, _height) = layout.get_size();
    cr.move_to(PAGE_WIDTH - points_from_inches(0.5), points_from_inches(0.5));
    cr.rel_move_to(points_from_pango(-width), 0.0);
    pc::show_layout(cr, &layout);
}

lazy_static! {
    static ref TITLE_FONT: FontDesc = {
        let mut font = BASE_FONT.clone();
        font.set_absolute_size(pango_from_points(20.0));
        font.set_weight(pango::Weight::Bold);
        FontDesc(font)
    };
}

fn draw_title(cr: &Cr, song: &Song) {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(&*TITLE_FONT);

    let title = song.title().expect("Song requires a title");

    layout.set_text(&title.text);
    layout.set_attributes(&title.formatting);
    pc::show_layout(cr, &layout);
    let (_width, height) = layout.get_size();
    cr.rel_move_to(0.0, 1.5 * points_from_pango(height));
}

#[derive(Clone, Debug)]
struct LayoutConfig<'a> {
    song: &'a Song,
    font_size: Points,
    verse_gap: Points,
    column_break: Option<Points>,
}

impl<'a> LayoutConfig<'a> {
    fn new(song: &'a Song) -> LayoutConfig<'a> {
        LayoutConfig {
            song,
            font_size: FONT_SIZE.into(),
            verse_gap: 14.0,
            column_break: None,
        }
    }

    fn shrink_h(&mut self) -> Result<(), ()> {
        if self.font_size > MIN_FONT_SIZE.into() {
            self.font_size -= 0.5;
            Ok(())
        } else {
            Err(())
        }
    }

    fn shrink_v(&mut self) -> Result<(), ()> {
        if self.font_size > MIN_FONT_SIZE.into() {
            self.font_size -= 0.5;
            Ok(())
        } else {
            Err(())
        }
    }
}

fn try_draw_verses(cr: &Cr, song: &Song) -> Result<(), ()> {
    let (start_x, start_y) = cr.get_current_point();
    let avail_width = PAGE_WIDTH - start_x - MARGIN_RIGHT;
    let avail_height = PAGE_HEIGHT - start_y - points_from_inches(0.5);
    let mut config = LayoutConfig::new(song);

    loop {
        cr.move_to(start_x, start_y);
        let (pat, size) = draw_verses(cr, &config);
        if size.width() > avail_width {
            config.shrink_h()?;
        } else if size.height() > avail_height {
            config.shrink_v()?;
        } else {
            cr.set_source(&*pat);
            cr.paint();
            return Ok(());
        }
    }
}

fn draw_verses(cr: &Cr, config: &LayoutConfig) -> (Box<cairo::Pattern>, Size) {
    let (start_x, start_y) = cr.get_current_point();
    let mut font = BASE_FONT.clone();
    font.set_absolute_size(pango_from_points(config.font_size));
    let mut max_width = Maximum::new(0.0);
    let mut max_height = Maximum::new(0.0);
    let mut height = 0.0;

    cr.push_group();
    for verse in &config.song.verses {
        if let Verse::RefrainDef(_, _) = *verse {} else {
            cr.rel_move_to(0.0, 14.0);
        }
        let Size(w, h) = draw_verse(&cr, &font, &verse);
        max_width.see(w);
        height += h + 14.0;
        if let Some(h) = config.column_break {
            if height > h {
                max_height.see(height);
                height = 0.0;
                cr.move_to(start_x + max_width.get() + GUTTER, start_y);
            }
        }
    }
    let pat = cr.pop_group();
    max_height.see(height);
    (pat, Size(max_width.get(), max_height.get()))
}

fn draw_verse(cr: &Cr, font: &FontDescription, verse: &Verse) -> Size {
    match *verse {
        Verse::Normal(ref lines) => {
            draw_lines(cr, &font, lines)
        },
        Verse::ChorusDef(ref label, ref lines) => {
            let label = &format!("{}:", label);
            let Size(label_w, label_h) = draw_label(cr, &font, label);
            cr.rel_move_to(0.0, label_h);
            let Size(body_w, body_h) = draw_lines(cr, &font, lines);
            Size(label_w.max(body_w), label_h + body_h)
        },
        Verse::RefrainDef(ref label, ref lines) => {
            let label = &format!("{}: ", label);
            let Size(label_w, _) = draw_label(cr, &font, label);
            cr.rel_move_to(label_w, 0.0);
            let Size(body_w, body_h) = draw_lines(cr, &font, lines);
            cr.rel_move_to(-label_w, 0.0);
            Size(label_w + body_w, body_h)
        },
        Verse::ChorusRef(ref label) => {
            draw_marker(cr, &font, label)
        },
        Verse::SectionBreak(ref label) => {
            draw_marker(cr, &font, label)
        },
    }
}

fn draw_lines(cr: &Cr, font: &FontDescription, lines: &[FormattedText])
-> Size {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);
    let mut max_width = Maximum::new(0.0);
    let mut total_height = 0.0;

    for line in lines {
        let indent = f64::from(line.indent) * INDENT;
        cr.rel_move_to(indent, 0.0);

        layout.set_text(&line.text);
        layout.set_attributes(&line.formatting);
        pc::show_layout(cr, &layout);

        cr.rel_move_to(-indent, 0.0);

        let (line_width, line_height) = layout.get_size();
        max_width.see(indent + points_from_pango(line_width));
        cr.rel_move_to(0.0, points_from_pango(line_height));
        total_height += points_from_pango(line_height);
    }
    Size(max_width.get(), total_height)
}

fn draw_label(cr: &Cr, font: &FontDescription, label: &str) -> Size {
    let bold = pango::AttrList::new();
    bold.insert(pango::Attribute::new_weight(pango::Weight::Bold).unwrap());

    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);

    layout.set_text(label);
    layout.set_attributes(&bold);
    pc::show_layout(cr, &layout);

    let size: Size = layout.get_size().into();
    size.map(points_from_pango)
}

fn draw_marker(cr: &Cr, font: &FontDescription, label: &str) -> Size {
    let italic = pango::AttrList::new();
    italic.insert(pango::Attribute::new_style(pango::Style::Italic).unwrap());

    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);

    layout.set_text(label);
    layout.set_attributes(&italic);
    pc::show_layout(cr, &layout);

    let size_: Size = layout.get_size().into();
    let size = size_.map(points_from_pango);
    cr.rel_move_to(0.0, size.height());
    size
}
