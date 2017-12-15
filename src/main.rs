use std::fs;
use std::path::Path;
use std::rc::Rc;

extern crate pango;

extern crate pest;
#[macro_use] extern crate pest_derive;
use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest::inputs::{FileInput, Input};

// Make Cargo aware of the dependency on grammar.pest
#[cfg(debug_assertions)]
const _GRAMMAR: &'static str = include_str!("grammar.pest");

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct SongParser;

fn main() {
    let dir = Path::new("/home/psimonyi/prj/Songs/typesetting");
    assert!(dir.is_dir());
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let input = FileInput::new(path).unwrap();
        let mut pairs = SongParser::parse(Rule::songsheet, Rc::new(input))
            .unwrap_or_else(|e| panic!("{}", e));
        println!("*** {} ***", entry.file_name().to_string_lossy());
        println!("{:?}", parse_song(&mut pairs));
    }
}

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

type Line = FormattedText;

struct FormattedText {
    text: String,
    formatting: pango::AttrList,
}
impl std::fmt::Debug for FormattedText {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "FormattedText({:?})", self.text)
    }
}

fn parse_song<I: Input>(src: &mut Pairs<Rule, I>) -> Song {
    let head = src.next().unwrap().into_inner();
    let body = src.next().unwrap().into_inner();
    assert_eq!(None, src.next());
    Song { meta: get_metadata(head), verses: get_verses(body) }
}

/// Collect the metadata from the head section (head rule).
/// The head is just a series of sexps (the separators don't make it into the
/// parse tree).
fn get_metadata<I: Input>(src: Pairs<Rule, I>) -> Vec<Metadata> {
    src.map(|sexp_pair| {
        assert_eq!(sexp_pair.as_rule(), Rule::sexp);
        let mut body = sexp_pair.into_inner();
        let keyword_pair = body.next().unwrap();
        assert_eq!(keyword_pair.as_rule(), Rule::keyword);
        let keyword = keyword_pair.as_str();
        make_meta_entry(keyword, body)
    }).collect()
}

fn make_meta_entry<I>(keyword: &str, args: Pairs<Rule, I>) -> Metadata
where I: Input {
    match keyword {
        "title" => Metadata::Title(parse_formatted_text(args)),
        "alt-title" => Metadata::AltTitle(parse_formatted_text(args)),
        _ => Metadata::Unknown,
        //_ => panic!("Unrecognized keyword"),
    }
}

fn parse_formatted_text<I: Input>(src: Pairs<Rule, I>) -> FormattedText {
    let mut ft = FormattedText {
        text: String::new(),
        formatting: pango::AttrList::new(),
    };
    add_formatted_text(src, &mut ft);
    ft
}

fn add_formatted_text<I: Input>(src: Pairs<Rule, I>, ft: &mut FormattedText) {
    for pair in src {
        match pair.as_rule() {
            Rule::text => ft.text.push_str(pair.as_str()),
            Rule::sexp => {
                let (keyword, body) = sexp_parts(pair);
                match keyword.as_str() {
                    "italic" => {
                        // When would new_style ever return None???
                        let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
                        attr.set_start_index(ft.text.len() as u32);
                        add_formatted_text(body, ft);
                        attr.set_end_index(ft.text.len() as u32);
                        ft.formatting.change(attr);
                    },
                    _ => panic!("bad keyword {}", keyword),
                }
            },
            _ => panic!("Why does the grammar allow that here"),
        }
    }
}

fn sexp_parts<'a, I: 'a + Input>(sexp: Pair<Rule, I>)
-> (String, Pairs<Rule, I>) {
    let mut body = sexp.into_inner();
    let keyword = body.next().unwrap();
    (keyword.as_str().into(), body)
}

/// Handle 'body' rule: contains only 'block's, which contain only 'line's.
fn get_verses<I: Input>(src: Pairs<Rule, I>) -> Vec<Verse> {
    src.map(|pair| parse_verse(pair.into_inner())).collect()
}

fn parse_verse<I: Input>(src: Pairs<Rule, I>) -> Verse {
    let label = String::new();
    let style = VerseType::Numbered;

    Verse {
        lines: src.map(|pair| parse_formatted_text(pair.into_inner())).collect(),
        label,
        style,
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
