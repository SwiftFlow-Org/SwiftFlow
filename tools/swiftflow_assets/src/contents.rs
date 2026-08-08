use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contents {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageEntry>,
    pub info: Info,

    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageEntry {

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub idiom: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub author: String,
    pub version: u32,
}

impl Default for Info {
    fn default() -> Self {

        Self {
            author: "xcode".to_string(),
            version: 1,
        }
    }
}

impl Contents {

    pub fn catalog_root() -> Self {
        Self {
            images: Vec::new(),
            info: Info::default(),
            extra: serde_json::Map::new(),
        }
    }

    pub fn empty_image_set() -> Self {
        Self {
            images: ["1x", "2x", "3x"]
                .iter()
                .map(|scale| ImageEntry {
                    filename: None,
                    idiom: "universal".to_string(),
                    scale: Some(scale.to_string()),
                    extra: serde_json::Map::new(),
                })
                .collect(),
            info: Info::default(),
            extra: serde_json::Map::new(),
        }
    }

    pub fn parse(text: &str) -> serde_json::Result<Self> {
        serde_json::from_str(text)
    }

    pub fn to_xcode_json(&self) -> serde_json::Result<String> {
        let mut out = Vec::new();
        let mut ser = serde_json::Serializer::with_formatter(&mut out, XcodeFormatter::new());
        self.serialize(&mut ser)?;
        let mut text = String::from_utf8(out).expect("serde_json emits UTF-8");
        text.push('\n');
        Ok(text)
    }
}

struct XcodeFormatter {
    indent: usize,
    has_value: bool,
}

impl XcodeFormatter {
    fn new() -> Self {
        Self {
            indent: 0,
            has_value: false,
        }
    }

    fn write_indent<W: ?Sized + io::Write>(&self, w: &mut W) -> io::Result<()> {
        for _ in 0..self.indent {
            w.write_all(b"  ")?;
        }
        Ok(())
    }
}

impl serde_json::ser::Formatter for XcodeFormatter {
    fn begin_array<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.indent += 1;
        self.has_value = false;
        w.write_all(b"[")
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.indent -= 1;
        if self.has_value {
            w.write_all(b"\n")?;
            self.write_indent(w)?;
        }
        w.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        w: &mut W,
        first: bool,
    ) -> io::Result<()> {
        w.write_all(if first { b"\n" } else { b",\n" })?;
        self.write_indent(w)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, _w: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.indent += 1;
        self.has_value = false;
        w.write_all(b"{")
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.indent -= 1;
        if self.has_value {
            w.write_all(b"\n")?;
            self.write_indent(w)?;
        }
        w.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + io::Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        w.write_all(if first { b"\n" } else { b",\n" })?;
        self.write_indent(w)
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_all(b" : ")
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, _w: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XCODE_IMAGESET: &str = r#"{
  "images" : [
    {
      "filename" : "logo.png",
      "idiom" : "universal",
      "scale" : "1x"
    },
    {
      "idiom" : "universal",
      "scale" : "2x"
    },
    {
      "filename" : "logo@3x.png",
      "idiom" : "universal",
      "scale" : "3x"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;

    #[test]
    fn xcode_json_round_trips_byte_for_byte() {
        let parsed = Contents::parse(XCODE_IMAGESET).expect("should parse");
        assert_eq!(parsed.to_xcode_json().unwrap(), XCODE_IMAGESET);
    }

    #[test]
    fn empty_slots_stay_absent_not_null() {
        let parsed = Contents::parse(XCODE_IMAGESET).unwrap();
        let two_x = parsed
            .images
            .iter()
            .find(|i| i.scale.as_deref() == Some("2x"))
            .unwrap();
        assert_eq!(two_x.filename, None);

        assert!(!parsed.to_xcode_json().unwrap().contains("null"));
    }

    #[test]
    fn unknown_keys_survive_a_round_trip() {

        let source = r#"{
  "images" : [
    {
      "appearances" : [
        {
          "appearance" : "luminosity",
          "value" : "dark"
        }
      ],
      "filename" : "logo-dark.png",
      "idiom" : "universal",
      "scale" : "1x"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"#;
        let parsed = Contents::parse(source).expect("should parse");
        assert!(parsed.to_xcode_json().unwrap().contains("luminosity"));
    }

    #[test]
    fn a_new_image_set_has_three_empty_universal_slots() {
        let set = Contents::empty_image_set();
        assert_eq!(set.images.len(), 3);
        assert!(set.images.iter().all(|i| i.filename.is_none()));
        assert!(set.images.iter().all(|i| i.idiom == "universal"));
    }
}
