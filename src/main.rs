use clap::Parser;
use quick_xml::de::from_str;
use quick_xml::events::BytesDecl;
use quick_xml::events::Event;
use quick_xml::Writer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

mod serialize_as_string {
    use csv::{self, Terminator};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(d: &Vec<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut res = String::new();

        let len = d.len();
        for (i, record) in d.iter().enumerate() {
            let mut v = Vec::new();
            let mut w = csv::WriterBuilder::new()
                .has_headers(false)
                .terminator(Terminator::Any(',' as u8))
                .from_writer(&mut v);
            w.serialize(record).map_err(serde::ser::Error::custom)?;
            drop(w);
            if i == len - 1 {
                v.pop();
            }
            let mut s = String::from_utf8(v).map_err(serde::ser::Error::custom)?;
            if i != len - 1 {
                s.push_str("\n");
            }
            res.push_str(&s);
        }
        serializer.serialize_str(&res)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u32>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut res: Vec<Vec<u32>> = Vec::new();
        let s = String::deserialize(deserializer)?;
        let s = s.split(",\n");
        for s in s {
            let mut r = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(s.as_bytes());
            let vals = r.records();
            let vals = vals.map(|v| v.unwrap());
            let vals = vals.map(|v| {
                v.iter()
                    .map(|v| v.parse::<u32>().unwrap())
                    .collect::<Vec<_>>()
            });
            res.append(&mut vals.collect::<Vec<_>>());
        }
        Ok(res)
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input .tmx file
    file: PathBuf,
    /// Tile to find
    find: u32,
    /// Tile to replace with
    replace: u32,

    /// Save result to file itself
    #[arg(short, long)]
    in_place: bool,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Image {
    #[serde(rename = "@source")]
    source: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Export {
    #[serde(rename = "@target")]
    target: String,
    #[serde(rename = "@format")]
    format: String,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct EditorSettings {
    export: Export,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TileSet {
    #[serde(rename = "@firstgid")]
    firstgid: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@tilewidth")]
    tilewidth: u32,
    #[serde(rename = "@tileheight")]
    tileheight: u32,
    #[serde(rename = "@tilecount")]
    tilecount: u32,
    #[serde(rename = "@columns")]
    columns: u32,
    image: Image,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
struct Data {
    #[serde(rename = "@encoding")]
    encoding: String,
    #[serde(rename = "$text", with = "serialize_as_string")]
    data: Vec<Vec<u32>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
struct Layer {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,
    #[serde(rename = "@offsetx", default)]
    offsetx: u32,
    #[serde(rename = "@offsety", default)]
    offsety: u32,
    data: Data,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Map {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@tiledversion")]
    tiledversion: String,
    #[serde(rename = "@orientation")]
    orientation: String,
    #[serde(rename = "@renderorder")]
    renderorder: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,
    #[serde(rename = "@tilewidth")]
    tilewidth: u32,
    #[serde(rename = "@tileheight")]
    tileheight: u32,
    #[serde(rename = "@infinite")]
    infinite: u32,
    #[serde(rename = "@backgroundcolor")]
    backgroundcolor: String,
    #[serde(rename = "@nextlayerid")]
    nextlayerid: u32,
    #[serde(rename = "@nextobjectid")]
    nextobjectid: u32,
    editorsettings: EditorSettings,
    tileset: TileSet,
    layer: Vec<Layer>,
}

fn main() {
    let cli = Cli::parse();

    let contents =
        fs::read_to_string(cli.file).expect("Should have been able to read the file");
    let mut map: Map = from_str(&contents).unwrap();
    for layer in &mut map.layer {
        for row in &mut layer.data.data.iter_mut() {
            for cell in row.iter_mut() {
                if *cell == cli.find {
                    *cell = cli.replace;
                }
            }
        }
    }

    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), ' ' as u8, 1);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .expect("cannot write xml header");
    writer
        .write_serializable("map", &map)
        .expect("cannot serialize map");
    let xml = writer.into_inner().into_inner();
    let xml_str = String::from_utf8_lossy(&xml);
    println!("{}", xml_str);
}
