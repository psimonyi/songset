use std::path::Path as FsPath;

use Song;
use Verse;

extern crate cairo;
extern crate pango;
extern crate pangocairo;

use self::pangocairo::functions as pc;
use pango::LayoutExt;

type Cr = cairo::Context;

const PAGE_WIDTH: f64 = 8.5 * 72f64; // points
const PAGE_HEIGHT: f64 = 11.0 * 72f64; // points

pub fn pdf_song(path: &FsPath, song: &Song) {
    let surface = cairo::PDFSurface::create(path, PAGE_WIDTH, PAGE_HEIGHT);
    let cr = cairo::Context::new(&surface);

    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.move_to(72.0, 72.0);
    draw_title(&cr, song);
    cr.rel_move_to(0.0, 28.0);

    for verse in &song.verses {
        draw_verse(&cr, verse);
        cr.rel_move_to(0.0, 28.0);
    }

    cr.show_page();
}

fn base_layout(cr: &Cr) -> pango::Layout {
    let mut fd = pango::FontDescription::new();
    fd.set_family("Caladea");
    fd.set_absolute_size(14.0 * pango::SCALE as f64);

    let layout = pc::create_layout(&cr).unwrap();
    layout.set_font_description(&fd);
    layout
}

fn draw_title(cr: &Cr, song: &Song) {
    let layout = base_layout(cr);
    let title = song.title().expect("Song requires a title");

    layout.set_text(&title.text);
    layout.set_attributes(&title.formatting);
    layout.get_attributes().unwrap().change(
        pango::Attribute::new_weight(pango::Weight::Bold).unwrap());
    pc::show_layout(cr, &layout);
}

const INDENT: f64 = 24.0; /* points */
fn draw_verse(cr: &Cr, verse: &Verse) {
    let layout = base_layout(cr);
    for line in &verse.lines {
        cr.rel_move_to(line.indent as f64 * INDENT, 0.0);

        layout.set_text(&line.text);
        layout.set_attributes(&line.formatting);

        pc::show_layout(cr, &layout);
        cr.rel_move_to(0.0, 14.0);
        cr.rel_move_to(-(line.indent as f64) * INDENT, 0.0);
    }
    cr.rel_move_to(0.0, -14.0);
}
