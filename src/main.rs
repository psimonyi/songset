use std::fs;
use std::io::Read;
use std::path::Path;

extern crate pango;

mod parse {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

fn main() {
    let dir = Path::new("/home/psimonyi/prj/Songs/typesetting");
    assert!(dir.is_dir());
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        println!("*** {} ***", entry.file_name().to_string_lossy());
        let path = entry.path();
        let mut file = fs::File::open(path).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        let parsed = parse::song(&s).unwrap();
        let song = tr_song(&parsed);

        println!("{:?}", song);
    }
}

// Structs produced by the parser:

#[derive(Debug)]
pub struct Line<'a> {
    indent: &'a str,
    items: Vec<Item<'a>>,
}

#[derive(Debug)]
struct Sexp<'a> {
    keyword: &'a str,
    items: Vec<Item<'a>>,
}

#[derive(Debug)]
enum Item<'a> {
    Text(&'a str),
    Sexp(Sexp<'a>),
}

// Structs produced after translation:

#[derive(Debug)]
struct Song {
    meta: Vec<Metadata>,
    verses: Vec<Verse>,
}

#[derive(Debug)]
struct Verse {
    lines: Vec<FormattedText>,
    style: VerseType,
}

#[derive(Debug)]
enum VerseType {
    Normal,
    Refrain(String), // e.g. "Chorus:"
    ChorusInstance(String), // e.g. "Chorus"
}

struct FormattedText {
    text: String,
    formatting: pango::AttrList,
    indent: u32,
}
impl std::fmt::Debug for FormattedText {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FormattedText({:?}, {})", self.text, self.indent)
    }
}

#[derive(Debug)]
enum Metadata {
    /// The primary title of the song.
    Title(FormattedText),
    /// An alternative title.
    AltTitle(FormattedText),
    /*
    /// RFC5646 language tag: the language of the text.
    Language(String),
    /// Reference to another book containing the song.
    CrossRef(FormattedText),
    /// Attribution.
    Attrib(FormattedText),
    /// This song has a descant (somewhere).
    Descant,
    /// The category in which to file this song.
    Category(String),
    /// Additional phrases under which to index this song.
    IndexEntry(String),
    */
    Unknown,
}

fn tr_song(src: &Vec<Vec<Line>>) -> Result<Song, Error> {
    let mut i = src.iter();
    let meta = tr_meta_block(i.next().unwrap());
    let verses = i.filter_map(tr_verse).collect();

    Song { meta, verses }
}

fn tr_meta_block(src: &Vec<Line>) -> Vec<Metadata> {
    src.iter().flat_map(|l| &l.items).filter_map(|item| {
        match *item {
            Item::Text(s) => {
                assert!(str_is_whitespace(s));
                None
            },
            Item::Sexp(ref sexp) => {
                Some(tr_meta_entry(sexp))
            },
        }
    }).collect()
}

fn str_is_whitespace(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

fn tr_meta_entry(sexp: &Sexp) -> Metadata {
    match sexp.keyword {
        "title" => Metadata::Title(tr_formatted_text(&sexp.items)),
        "alt-title" => Metadata::AltTitle(tr_formatted_text(&sexp.items)),
        _ => Metadata::Unknown,
        //_ => panic!("Unrecognized keyword"),
    }
}

fn tr_verse(src: &Vec<Line>) -> Option<Verse> {
    let mut i = src.iter().peekable();
    let style = {
        let line = i.peek().unwrap(); // It's the first; there must be one.
        tr_verse_meta(line)
    };
    if style.is_some() {
        // We used the peeked line so advance the iterator.
        i.next();
    }

    let lines = i.map(tr_line).collect();
    Some(Verse {
        lines,
        style: style.unwrap_or(VerseType::Normal),
    })
}

fn tr_verse_meta(line: &Line) -> Option<VerseType> {
    let mut rv = None;
    for item in &line.items {
        match *item {
            Item::Text(ref s) => {
                if !str_is_whitespace(s) { return None; }
            },
            Item::Sexp(ref sexp) => {
                if rv.is_some() { return None; }
                match *sexp {
                    Sexp { keyword: "Chorus:", ref items }
                    if items.is_empty() => {
                        rv = Some(VerseType::Refrain(String::from("Chorus")));
                    },
                    Sexp { keyword: "Refrain:", ref items }
                    if items.is_empty() => {
                        rv = Some(VerseType::Refrain(String::from("Refrain")));
                    },
                    Sexp { keyword: "Chorus", ref items }
                    if items.is_empty() => {
                        rv = Some(VerseType::ChorusInstance(String::from("Chorus")));
                    },
                    _ => return None,
                }
            },
        }
    }
    rv
}

fn tr_line(src: &Line) -> FormattedText {
    tr_formatted_text(&src.items)
}

fn tr_formatted_text(src: &Vec<Item>) -> FormattedText {
    let mut ft = FormattedText {
        text: String::new(),
        formatting: pango::AttrList::new(),
        indent: 0,
    };
    add_formatted_text(src, &mut ft);
    ft
}

fn add_formatted_text(src: &Vec<Item>, ft: &mut FormattedText) {
    for item in src {
        match *item {
            Item::Text(ref s) => ft.text.push_str(s),
            Item::Sexp(Sexp{keyword: "italic", ref items}) => {
                // When would new_style ever return None???
                let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
                attr.set_start_index(ft.text.len() as u32);
                add_formatted_text(items, ft);
                attr.set_end_index(ft.text.len() as u32);
                ft.formatting.change(attr);
            },
            ref x => panic!("bad item {:?}", x),
        }
    }
}

/*
author - should just use attrib
dance - i marked a few waltzes
origin - english, irish, etc
source - where I got the words (lark in the crear air)
white-book - translate to CrossRef
white-book-title - translate to AltTitle
}
*/
//
// Recognized sexp keywords:
/*
Text formatting
===============
...  replace with ellipsis
footnote
italic
math

Verse info
==========
Chorus
Chorus:
Refrain:
refrain
verse-label
section-break

Metadata
========
alt-title
attrib
author
category
dance
descant  means it has a descant
index
lang
origin
ref
source
title
white-book
white-book-note
white-book-title

Misc
====
ignore-this-file
inline-chorus
inline-chorus-markers
note
numbered-verses
TODO
todo
TODO-special-formatting

*/
