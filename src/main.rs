use serde::{Serialize, Deserialize};
use std::path::{Path,PathBuf};
use clap::{Arg, App};

fn main() -> std::io::Result<()> {

  let matches = App::new("hakyll2zola")
                  .version("1.0")
                  .author("Yutaka Imamura <ilyaletre@gmail.com>")
                  .about("convert hakyll doc to zola doc")
                  .arg(Arg::with_name("input").long("input").short('i').required(true).takes_value(true))
                  .arg(Arg::with_name("output").long("output").short('o').required(true).takes_value(true))
                  .arg(Arg::with_name("alias").long("alias").short('a').required(true).takes_value(true))
                  .get_matches();

  let mut alias_path: PathBuf = PathBuf::from(matches.value_of("alias").unwrap());
  let input: &Path = Path::new(matches.value_of("input").unwrap());
  let output: &Path = Path::new(matches.value_of("output").unwrap());

  let content = std::fs::read_to_string(input)?;
  let mut stream = Stream::new(&content);
  match stream.read_header() {
    Ok(mut metadata) => {
      alias_path.push(input.with_extension("html").file_name().unwrap());
      metadata.alias = Some(alias_path.as_os_str().to_str().unwrap().to_string());
      let mut buf = String::new();
      buf.push_str(&metadata.format_header());
      buf.push_str(stream.current());
      std::fs::create_dir_all(output.parent().unwrap())?;
      std::fs::write(output, buf)?;
      Ok(())
    },
    Err(e) => {
      println!("{:?}", e);
      Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))
    },
  }
}

fn print_list_toml(elements: Vec<&str>) -> String {
  let mut buf = String::new();
  buf.push('[');
  let str_elements: Vec<String> = elements.iter().map(|t| format!("\"{}\"", t)).collect();
  buf.push_str(str_elements.join(",").as_str());
  buf.push(']');
  buf
}

fn print_tags_as_toml(tags: &String) -> String {
  let mut tag_vec = Vec::new();
  tags.split(", ").for_each(|tag| {
    tag_vec.push(tag)
  });
  print_list_toml(tag_vec)
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Metadata {
  title: String,
  date: Option<String>,
  tags: Option<String>,
  alias: Option<String>,
}

impl Metadata {
  fn format_header(&self) -> String {
    let mut buf = String::new();
    buf.push_str("+++\n");
    buf.push_str(format!("title = \"{}\"\n", &self.title).as_str());
    if let Some(date) = &self.date {
      buf.push_str(format!("date = {}\n", date).as_str());
    }
    if let Some(alias) = &self.alias {
      buf.push_str(format!("aliases = [\"{}\"]\n", alias).as_str());
    }
    if let Some(tags) = &self.tags {
      buf.push_str("[taxonomies]\n");
      buf.push_str(format!("tags = {}\n", print_tags_as_toml(tags)).as_str());
    }
    buf.push_str("+++");
    buf
  }
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
  use super::{Stream, Metadata};
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
      alias: None,
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
      alias: None,
    });
  }
}