use std::str::Chars;
use std::iter::Peekable;
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

fn main() -> std::io::Result<()> {
  let mut buf = String::new();
  // read all bytes
  loop {
    let read = std::io::stdin().read_line(&mut buf)?;
    if read == 0 {
      break;
    }
  }
  let mut stream = Stream::new(&buf);
  match stream.read_header() {
    Ok(metadata) => {
      println!("+++");
      println!("title = \"{}\"", &metadata.title);
      if let Some(date) = &metadata.date {
        println!("date = {}", date);
      }
      if let Some(tags) = &metadata.tags {
        println!("[taxonomies]");
        println!("tags = {}", print_tags_as_toml(tags));
      }
      print!("+++");
      print!("{}", stream.current());
    },
    Err(e) => println!("{:?}", e),
  }
  Ok(())
}

fn print_tags_as_toml(tags: &String) -> String {
  let mut tag_vec = Vec::new();
  let mut buf = String::new();
  buf.push('[');
  tags.split(", ").for_each(|tag| {
    tag_vec.push(tag)
  });
  let tag_str_literars: Vec<String> = tag_vec.iter().map(|t| format!("\"{}\"", t)).collect();
  buf.push_str(tag_str_literars.join(",").as_str());
  buf.push(']');
  buf
}
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Metadata {
  title: String,
  date: Option<String>,
  tags: Option<String>,
}

#[derive(Debug)]
enum ParseError {
  BadSyntax(String),
  WrongYaml(serde_yaml::Error),
}

impl From<serde_yaml::Error> for ParseError {
  fn from(e: serde_yaml::Error) -> Self {
    ParseError::WrongYaml(e)
  }
}

type ParseResult<T> = Result<T, ParseError>;

struct Stream<'a> {
  offset: usize,
  content: &'a String
}

impl <'a> Stream<'a> {
  fn new(content: &'a String) -> Stream<'a> {
    let ch = content.chars();
    Stream {
      offset: 0,
      content: content,
    }
  }

  fn read_header(&mut self) -> ParseResult<Metadata> {
    let _start_mark = self.read_string("---")?;
    let metadata_raw = self.read_until("---")?;
    let _end_mark = self.read_string("---")?;
    let metadata = serde_yaml::from_str(metadata_raw)?;
    Ok(metadata)
  }

  fn current(&self) -> &'a str {
    self.content.get(self.offset..).unwrap()
  }

  fn read_string(&mut self, s: &str) -> ParseResult<&'a str> {
    let len = s.len();
    match self.current().get(0..len) {
      Some(sub) => {
        if s == sub {
          self.offset = self.offset + len;
          Ok(sub)
        }
        else {
          Err(ParseError::BadSyntax(format!("expected {:?} but got {:?}", s, sub)))
        }
      },
      None => Err(ParseError::BadSyntax(format!("unexpected end of input to read {:?}", s)))
    }
  }

  fn read_until(&mut self, s: &str) -> ParseResult<&'a str> {
    match self.current().find(s) {
      Some(idx) => {
        let sliced = self.current().get(0..idx).unwrap();
        self.offset = self.offset + idx;
        Ok(sliced)
      },
      None => Err(ParseError::BadSyntax(format!("expected \"{:?}\" but not found", s))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{Stream, Metadata, ParseError};
  #[test]
  fn test_read_string() {
    let s = String::from("---\ntitle: タイトル");
    let mut stream = Stream::new(&s);
    assert_eq!(stream.read_string("---").unwrap(), "---");
  }

  #[test]
  fn test_read_until() {
    let s = String::from("\ntitle: タイトル\n---\n");
    let mut stream = Stream::new(&s);
    assert_eq!(stream.read_until("---").unwrap(), "\ntitle: タイトル\n");
    assert_eq!(stream.read_string("---").unwrap(), "---");
  }

  #[test]
  fn test_read_header() {
    let s = String::from("---\ntitle: タイトル\n---");
    let mut stream = Stream::new(&s);
    assert_eq!(stream.read_header().unwrap(), Metadata {
      title: String::from("タイトル"),
      date: None,
      tags: None,
    });
  }

  #[test]
  fn test_read_header_full() {
    let s = String::from("---\ntitle: 『ビッグデータを支える技術』を読んだ データインジェスチョンについて\ndate: 2020-02-01\ntags: database, book\n---\nbody");
    let mut stream = Stream::new(&s);
    assert_eq!(stream.read_header().unwrap(), Metadata {
      title: String::from("『ビッグデータを支える技術』を読んだ データインジェスチョンについて"),
      date: Some(String::from("2020-02-01")),
      tags: Some(String::from("database, book")),
    });
  }
}