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
const FONT_SIZE: Points = 16.0;
const MIN_FONT_SIZE: Points = 13.0;

fn points_from_inches(size: f64) -> f64 {
    size * 72.0
}
fn points_from_pango(size: i32) -> f64 {
    size as f64 / pango::SCALE as f64
}
fn pango_from_points(size: f64) -> f64 {
    size * pango::SCALE as f64
}

pub fn pdf_song(path: &FsPath, song: &Song) {
    let surface = cairo::PDFSurface::create(path, PAGE_WIDTH, PAGE_HEIGHT);
    let cr = cairo::Context::new(&surface);

    let mut title_font = pango::FontDescription::new();
    title_font.set_family("Caladea");
    title_font.set_absolute_size(pango_from_points(20.0));
    title_font.set_weight(pango::Weight::Bold);

    cr.move_to(points_from_inches(1.5), points_from_inches(0.5));
    draw_title(&cr, &title_font, song);
    cr.rel_move_to(0.0, 20.0*1.25*2.0);

    for verse in &song.verses {
        draw_verse(&cr, verse);
        cr.rel_move_to(0.0, 28.0);
    }

    cr.show_page();
}

fn draw_title(cr: &Cr, font: &FontDescription, song: &Song) {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);

    let title = song.title().expect("Song requires a title");

    layout.set_text(&title.text);
    layout.set_attributes(&title.formatting);
    pc::show_layout(cr, &layout);
}

fn draw_verse(cr: &Cr, verse: &Verse) {
    let mut verse_font = pango::FontDescription::new();
    verse_font.set_family("Caladea");
    verse_font.set_absolute_size(pango_from_points(FONT_SIZE));
    let mut label_font = verse_font.clone();
    label_font.set_weight(pango::Weight::Bold);
    let mut alone_font = verse_font.clone();
    alone_font.set_style(pango::Style::Italic);

    match *verse {
        Verse::Normal(ref lines) => {
            draw_lines(cr, &verse_font, lines);
        },
        Verse::ChorusDef(ref label, ref lines) => {
            let label = &format!("{}:", label);
            let (_width, height) = draw_label(cr, &label_font, label);
            cr.rel_move_to(0.0, points_from_pango(height));
            draw_lines(cr, &verse_font, lines);
        },
        Verse::RefrainDef(ref label, ref lines) => {
            let label = &format!("{}: ", label);
            let (width, _height) = draw_label(cr, &label_font, label);
            cr.rel_move_to(points_from_pango(width), 0.0);
            draw_lines(cr, &verse_font, lines);
            cr.rel_move_to(-points_from_pango(width), 0.0);
        },
        Verse::ChorusRef(ref label) => {
            draw_label(cr, &alone_font, label);
        },
        Verse::SectionBreak(ref label) => {
            draw_label(cr, &alone_font, label);
        },
    }
}

fn draw_lines(cr: &Cr, font: &FontDescription, lines: &[FormattedText]) {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);

    for line in lines {
        cr.rel_move_to(line.indent as f64 * INDENT, 0.0);

        layout.set_text(&line.text);
        layout.set_attributes(&line.formatting);
        pc::show_layout(cr, &layout);

        cr.rel_move_to(-(line.indent as f64) * INDENT, 0.0);

        let (_line_width, line_height) = layout.get_size();
        cr.rel_move_to(0.0, points_from_pango(line_height));
    }
    cr.rel_move_to(0.0, -14.0);
}

fn draw_label(cr: &Cr, font: &FontDescription, label: &str)
-> (i32, i32) {
    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(font);

    layout.set_text(label);
    pc::show_layout(cr, &layout);

    layout.get_size()
}
