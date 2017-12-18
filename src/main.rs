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

        println!("{:?}", parse::song(&s));
    }
}

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

/*
#[derive(Debug)]
struct Song {
    meta: Vec<Metadata>,
    verses: Vec<Verse>,
}

/*
 * a verse can be a list of lines, each of which is formatted text
 * the verse has a number or is a chorus or has a label
 * is the verse indented or are lines indented or both? probably lines.
 * formatted text is another matter though
 * and titles or other metadata could contain that
 */
#[derive(Debug)]
struct Verse {
    lines: Vec<Line>,
    label: String,
    style: VerseType,
}

#[derive(Debug)]
enum VerseType {
    Refrain,
    Numbered,
    Labelled,
}

struct FormattedText {
    text: String,
    formatting: pango::AttrList,
}
impl std::fmt::Debug for FormattedText {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FormattedText({:?})", self.text)
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

fn parse_formatted_text(src: &Vec<Item>) -> FormattedText {
    let mut ft = FormattedText {
        text: String::new(),
        formatting: pango::AttrList::new(),
    };
    add_formatted_text(src, &mut ft);
    ft
}

fn add_formatted_text(src: &Vec<Item>, ft: &mut FormattedText) {
    for item in src {
        match *item {
            Item::Text(ref s) => ft.text.push_str(s),
            Item::Sexp(ref s) => {
                match s.keyword {
                    "italic" => {
                        // When would new_style ever return None???
                        let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
                        attr.set_start_index(ft.text.len() as u32);
                        add_formatted_text(&s.items, ft);
                        attr.set_end_index(ft.text.len() as u32);
                        ft.formatting.change(attr);
                    },
                    _ => panic!("bad keyword {}", s.keyword),
                }
            },
        }
    }
}
*/

/*
fn make_meta_entry<I>(keyword: &str, args: Pairs<Rule, I>) -> Metadata
where I: Input {
    match keyword {
        "title" => Metadata::Title(parse_formatted_text(args)),
        "alt-title" => Metadata::AltTitle(parse_formatted_text(args)),
        _ => Metadata::Unknown,
        //_ => panic!("Unrecognized keyword"),
    }
}

*/

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
