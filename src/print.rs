use std::path::Path as FsPath;

use super::Song;

extern crate cairo;
extern crate pango;
extern crate pangocairo;

use self::pangocairo::functions as pc;
use pango::LayoutExt;

const PAGE_WIDTH: f64 = 8.5 * 72f64; // points
const PAGE_HEIGHT: f64 = 11.0 * 72f64; // points

pub fn pdf_song(path: &FsPath, song: &Song) {
    let surface = cairo::PDFSurface::create(path, PAGE_WIDTH, PAGE_HEIGHT);
    let cr = cairo::Context::new(&surface);

    let mut fd = pango::FontDescription::new();
    fd.set_family("Caladea");
    fd.set_absolute_size(12.0 * pango::SCALE as f64);

    let layout = pc::create_layout(&cr).expect("bwuh, null?");
    layout.set_font_description(&fd);
    let title = &song.title().unwrap().text;
    layout.set_text(title);

    cr.set_source_rgb(0.0, 0.0, 0.0);
    cr.move_to(72.0, 72.0);
    pc::show_layout(&cr, &layout);

    cr.show_page();
}
