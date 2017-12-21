use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

extern crate pango;

mod print;

mod parse {
    include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
}

fn main() {
    let dir = Path::new("/home/psimonyi/prj/Songs/typesetting");
    let out_dir = Path::new("/home/psimonyi/prj/Songs/out");
    assert!(dir.is_dir());
    assert!(out_dir.is_dir());

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        println!("*** {} ***", entry.file_name().to_string_lossy());
        let path = entry.path();
        match read_song(&path) {
            Err(e) => println!("Error: {}", e),
            Ok(song) => {
                let out_path = out_dir.join(entry.file_name());
                print::pdf_song(&out_path, &song);
            },
        }
    }
}

fn read_song(filepath: &Path) -> Result<Song, Error> {
    let mut file = fs::File::open(filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let parsed = parse::song(&contents)?;
    tr_song(&parsed)
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

impl<'a> fmt::Display for Sexp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let items = self.items.iter().map(|item| match *item {
            Item::Text(ref s) => format!("{}", s),
            Item::Sexp(ref sexp) => format!("{}", sexp),
        }).collect::<Vec<String>>().concat();
        write!(f, "⟦{} {}⟧", self.keyword, items)
    }
}

impl<'a> Sexp<'a> {
    fn string_item(&self) -> Result<&'a str, String> {
        if self.items.len() == 1 {
            match self.items[0] {
                Item::Text(s) => Ok(s),
                _ => Err(format!("Expected string argument in {:?}", self)),
            }
        } else {
            Err(format!("Exactly one argument required in {:?}", self))
        }
    }

    fn has_args(&self) -> bool {
        !self.items.is_empty()
    }
}

#[derive(Debug)]
enum Item<'a> {
    Text(&'a str),
    Sexp(Sexp<'a>),
}

// Structs produced after translation:

#[derive(Debug)]
pub struct Song {
    meta: Vec<Metadata>,
    verses: Vec<Verse>,
}

impl Song {
    fn title<'a>(&'a self) -> Option<&'a FormattedText> {
        for m in self.meta.iter() {
            if let Metadata::Title(ref t) = *m {
                return Some(t);
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct Verse {
    lines: Vec<FormattedText>,
    style: VerseType,
}

#[derive(Debug)]
enum VerseType {
    Normal,
    Refrain(String), // e.g. "Chorus:"
    ChorusInstance(String), // e.g. "Chorus"
    SectionBreak(String), // e.g. alt language or another poem to the same tune
}

struct FormattedText {
    text: String,
    formatting: pango::AttrList,
    indent: u32,
}
impl fmt::Debug for FormattedText {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FormattedText({:?}, {})", self.text, self.indent)
    }
}

impl FormattedText {
    fn new() -> FormattedText {
        FormattedText {
            text: String::new(),
            formatting: pango::AttrList::new(),
            indent: 0,
        }
    }
}

#[derive(Debug)]
struct Error (String);

impl Error {
    fn new<S: Into<String>>(message: S) -> Self {
        Error(message.into())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        &self.0
    }
}

impl From<parse::ParseError> for Error {
    fn from(error: parse::ParseError) -> Error {
        Error(format!("Parser: {}", error))
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error(format!("I/O error: {}", error))
    }
}

impl From<String> for Error {
    fn from(message: String) -> Error {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Translation error: {}", self.0)
    }
}

fn tr_song(src: &Vec<Vec<Line>>) -> Result<Song, Error> {
    let mut i = src.iter();
    let meta = tr_meta_block(i.next().unwrap())?;
    let verses = i.map(tr_verse).collect::<Result<_, _>>()?;
    let mut song = Song { meta, verses };
    normalize_indents(&mut song);

    Ok(song)
}

fn tr_meta_block(src: &Vec<Line>) -> Result<Vec<Metadata>, Error> {
    src.iter()
        .flat_map(|l| &l.items)
        .filter_map(|item| {
            match *item {
                Item::Text(ref s) if str_is_whitespace(s) =>
                    None,
                Item::Text(ref s) =>
                    Some(Err(Error::new(
                        format!("Text in the meta block: {:?}", s)))),
                Item::Sexp(ref sexp) =>
                    Some(tr_meta_entry(sexp)),
            }
        })
        .collect()
}

fn str_is_whitespace(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

fn tr_meta_entry(sexp: &Sexp) -> Result<Metadata, Error> {
    match sexp.keyword {
        "title" => Ok(Metadata::Title(tr_formatted_text(&sexp.items)?)),
        "alt-title" => Ok(Metadata::AltTitle(tr_formatted_text(&sexp.items)?)),
        "attrib" => Ok(Metadata::Attrib(tr_formatted_text(&sexp.items)?)),
        "ref" => Ok(Metadata::CrossRef(tr_formatted_text(&sexp.items)?)),

        // To be translated
        "white-book" => Ok(Metadata::Title(tr_formatted_text(&sexp.items)?)),
        "white-book-title" => Ok(Metadata::Title(tr_formatted_text(&sexp.items)?)),
        "author" => Ok(Metadata::Attrib(tr_formatted_text(&sexp.items)?)),

        "category" => Ok(Metadata::Category(sexp.string_item()?.into())),
        "index" => Ok(Metadata::IndexEntry(sexp.string_item()?.into())),
        "lang" => Ok(Metadata::Language(sexp.string_item()?.into())),
        "dance" => Ok(Metadata::Dance(sexp.string_item()?.into())),
        "descant" => {
            if sexp.has_args() {
                return Err(Error::new(format!(
                    "⟦descant⟧ takes no arguments: {:?}", sexp)));
            }
            Ok(Metadata::Descant)
        },
        "numbered-verses" => Ok(Metadata::Ignored),
        "todo" => Ok(Metadata::Ignored),
        "TODO" => Ok(Metadata::Ignored),
        "TODO-special-formatting" => Ok(Metadata::Ignored),
        "note" => Ok(Metadata::Ignored),
        "inline-chorus-markers" => Ok(Metadata::Ignored),
        "inline-chorus" => Ok(Metadata::Ignored),
        "white-book-note" => Ok(Metadata::Ignored),
        "origin" => Ok(Metadata::Ignored),
        "source" => Ok(Metadata::Ignored),
        k => Err(Error::new(format!("Unrecognized meta keyword {:?}", k))),
    }
}

#[derive(Debug)]
enum Metadata {
    /// The primary title of the song.
    Title(FormattedText),
    /// An alternative title.
    AltTitle(FormattedText),
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
    /// A type of dance this song may be suitable for.
    Dance(String),
    Ignored,
}


fn tr_verse(src: &Vec<Line>) -> Result<Verse, Error> {
    let mut i = src.iter().peekable();
    let style = {
        let line = i.peek().unwrap(); // It's the first; there must be one.
        tr_verse_meta(line)
    };
    if style.is_some() {
        // We used the peeked line so advance the iterator.
        i.next();
    }

    let lines = i.map(tr_line).collect::<Result<_,_>>()?;
    Ok(Verse {
        lines,
        style: style.unwrap_or(VerseType::Normal),
    })
}

fn tr_verse_meta(line: &Line) -> Option<VerseType> {
    // For ChorusInstance and SectionBreak we should also check that the rest
    // of the verse is empty...
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
                    Sexp { keyword: "refrain", .. } => {
                        rv = sexp.string_item().ok()
                            .map(String::from)
                            .map(VerseType::ChorusInstance);
                    },
                    Sexp { keyword: "section-break", .. } => {
                        rv = sexp.string_item().ok()
                            .map(String::from)
                            .map(VerseType::SectionBreak);
                    },
                    _ => return None,
                }
            },
        }
    }
    rv
}

fn tr_line(src: &Line) -> Result<FormattedText, Error> {
    let mut ft = tr_formatted_text(&src.items)?;
    ft.indent = src.indent.len() as u32;
    Ok(ft)
}

fn tr_formatted_text(src: &Vec<Item>) -> Result<FormattedText, Error> {
    let mut ft = FormattedText::new();
    add_formatted_text(src, &mut ft)?;
    Ok(ft)
}

fn add_formatted_text(src: &Vec<Item>, ft: &mut FormattedText)
-> Result<(), Error> {
    for item in src {
        match *item {
            Item::Text(ref s) => ft.text.push_str(s),
            Item::Sexp(Sexp{keyword: "italic", ref items}) |
            Item::Sexp(Sexp{keyword: "note", ref items}) => {
                // When would new_style ever return None???
                let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
                attr.set_start_index(ft.text.len() as u32);
                add_formatted_text(items, ft)?;
                attr.set_end_index(ft.text.len() as u32);
                ft.formatting.change(attr);
            },
            Item::Sexp(Sexp{keyword: "footnote", ref items}) => {
                // When would new_style ever return None???
                let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
                attr.set_start_index(ft.text.len() as u32);
                add_formatted_text(items, ft)?;
                attr.set_end_index(ft.text.len() as u32);
                ft.formatting.change(attr);
            },
            Item::Sexp(ref s @ Sexp{keyword: "...", ..}) => {
                if !s.items.is_empty() {
                    return Err(Error::new(format!(
                        "⟦... {:?}⟧ should have no arguments", s.items)));
                }
                ft.text.push_str("…");
            },
            Item::Sexp(ref sexp) => return Err(Error::new(format!(
                "Unrecognized formatting command '{}'", sexp))),
        }
    }
    Ok(())
}

fn normalize_indents(song: &mut Song) {
    let mut indents = std::collections::HashSet::new();
    for verse in &song.verses {
        for line in &verse.lines {
            indents.insert(line.indent);
        }
    }
    let mut sizes: Vec<&u32> = indents.iter().collect();
    sizes.sort();
    for verse in &mut song.verses {
        for line in &mut verse.lines {
            line.indent = sizes.iter().position(|i| **i == line.indent).unwrap() as u32;
        }
    }
}
