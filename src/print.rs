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

pub fn pdf_song(path: &FsPath, song: &Song) {
    let surface = cairo::PDFSurface::create(path, PAGE_WIDTH, PAGE_HEIGHT);
    let cr = cairo::Context::new(&surface);

    cr.move_to(points_from_inches(1.5), points_from_inches(0.5));
    draw_title(&cr, song);

    draw_verses(&cr, song);

    draw_file_letter(&cr, song);
    cr.show_page();
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

fn draw_verses(cr: &Cr, song: &Song) {
    let (start_x, start_y) = cr.get_current_point();

    // Font sizes in half-point decrements from FONT_SIZE down to
    // MIN_FONT_SIZE, inclusive:
    let font_sizes = ((2 * MIN_FONT_SIZE)..(2 * FONT_SIZE + 1))
        .map(|x| f64::from(x) / 2.0).rev();

    for font_size in font_sizes {
        cr.move_to(start_x, start_y);
        let (pat, size) = draw_verses_straight(cr, song, font_size);
        if start_x + size.width() + MARGIN_RIGHT <= PAGE_WIDTH {
            cr.set_source(&*pat);
            cr.paint();
            return;
        }
    }
    println!("Can't fit it all in!");
    cr.move_to(start_x, start_y);
    let (pat, _size) = draw_verses_straight(cr, song, MIN_FONT_SIZE.into());
    cr.set_source(&*pat);
    cr.paint();
}

fn draw_verses_straight(cr: &Cr, song: &Song, font_size: Points)
-> (Box<cairo::Pattern>, Size) {
    let mut font = BASE_FONT.clone();
    font.set_absolute_size(pango_from_points(font_size));
    let mut max_width = Maximum::new(0.0);
    let mut height = 0.0;

    cr.push_group();
    for verse in &song.verses {
        if let Verse::RefrainDef(_, _) = *verse {} else {
            cr.rel_move_to(0.0, 14.0);
        }
        let Size(w, h) = draw_verse(&cr, &font, &verse);
        max_width.see(w);
        height += h + 14.0;
    }
    let pat = cr.pop_group();
    (pat, Size(max_width.get(), height))
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
