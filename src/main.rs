use clap::{Parser, Subcommand};
use csv::{self, Terminator};
use quick_xml::de::from_str;
use quick_xml::events::BytesDecl;
use quick_xml::events::Event;
use quick_xml::Writer;
use serde::ser::SerializeSeq;
use serde::ser::SerializeStruct;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::marker::PhantomData;
use std::path::PathBuf;

pub trait SerializationFormat {
    fn serialize_data<'a, S>(items: &Vec<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
    fn transform_name(name: &str) -> &str;
    fn transform_vec_name(name: &str) -> &str;
}

pub struct XmlFormat;
impl SerializationFormat for XmlFormat {
    fn serialize_data<'a, S>(items: &Vec<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = String::new();

        let len = items.len();
        for (i, record) in items.iter().enumerate() {
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

    fn transform_name(name: &str) -> &str {
        name
    }

    fn transform_vec_name(name: &str) -> &str {
        let mut chars = name.chars();
        chars.next_back();
        chars.as_str()
    }
}

pub struct JsonFormat;
impl SerializationFormat for JsonFormat {
    fn serialize_data<'a, S>(items: &Vec<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ser = serializer.serialize_seq(None)?;
        for row in items {
            for cell in row {
                ser.serialize_element(cell)?;
            }
        }
        ser.end()
    }

    fn transform_name(name: &str) -> &str {
        if name.starts_with("@") {
            let mut chars = name.chars();
            chars.next();
            chars.as_str()
        } else {
            name
        }
    }

    fn transform_vec_name<'a>(name: &str) -> &str {
        name
    }
}

#[derive(Debug, Subcommand, PartialEq)]
enum Commands {
    /// Replace tile on all layers
    Replace {
        /// Tile to find
        find: u32,

        /// Tile to replace with
        replace: u32,
    },
    /// Resize tileset and update all tiles
    /// (old values are from tmx file)
    Resize { tilecount: u32, columns: u32 },
    /// Convert .tmx file to .json
    Convert,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input .tmx file
    file: PathBuf,

    #[command(subcommand)]
    command: Commands,

    /// Save result to file itself
    #[arg(short, long)]
    in_place: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct Image<T: SerializationFormat> {
    #[serde(rename = "@source")]
    source: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,
    #[serde(skip)]
    rest: PhantomData<T>,
}

impl<T> Serialize for Image<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = serializer.serialize_struct("image", 3)?;
        res.serialize_field(T::transform_name("@source"), &self.source)?;
        res.serialize_field(T::transform_name("@width"), &self.width)?;
        res.serialize_field(T::transform_name("@height"), &self.height)?;
        res.end()
    }
}

impl From<Image<XmlFormat>> for Image<JsonFormat> {
    fn from(image: Image<XmlFormat>) -> Self {
        Image::<JsonFormat> {
            source: image.source,
            width: image.width,
            height: image.height,
            rest: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct Export<T: SerializationFormat> {
    #[serde(rename = "@target")]
    target: String,
    #[serde(rename = "@format")]
    format: String,
    #[serde(skip)]
    rest: PhantomData<T>,
}

impl<T> Serialize for Export<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = serializer.serialize_struct("export", 2)?;
        res.serialize_field(T::transform_name("@target"), &self.target)?;
        res.serialize_field(T::transform_name("@format"), &self.format)?;
        res.end()
    }
}

impl From<Export<XmlFormat>> for Export<JsonFormat> {
    fn from(export: Export<XmlFormat>) -> Self {
        Export::<JsonFormat> {
            target: export.target,
            format: export.format,
            rest: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct EditorSettings<T: SerializationFormat> {
    export: Export<T>,
    #[serde(skip)]
    rest: PhantomData<T>,
}

impl From<EditorSettings<XmlFormat>> for EditorSettings<JsonFormat> {
    fn from(editorsettings: EditorSettings<XmlFormat>) -> Self {
        EditorSettings::<JsonFormat> {
            export: editorsettings.export.into(),
            rest: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct TileSet<T: SerializationFormat> {
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
    image: Image<T>,
}

impl From<TileSet<XmlFormat>> for TileSet<JsonFormat> {
    fn from(tileset: TileSet<XmlFormat>) -> Self {
        TileSet::<JsonFormat> {
            firstgid: tileset.firstgid,
            name: tileset.name,
            tilewidth: tileset.tilewidth,
            tileheight: tileset.tileheight,
            tilecount: tileset.tilecount,
            columns: tileset.columns,
            image: tileset.image.into(),
        }
    }
}

impl<T> Serialize for TileSet<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = serializer.serialize_struct("tileset", 7)?;
        res.serialize_field(T::transform_name("@firstgid"), &self.firstgid)?;
        res.serialize_field(T::transform_name("@name"), &self.name)?;
        res.serialize_field(T::transform_name("@tilewidth"), &self.tilewidth)?;
        res.serialize_field(T::transform_name("@tileheight"), &self.tileheight)?;
        res.serialize_field(T::transform_name("@tilecount"), &self.tilecount)?;
        res.serialize_field(T::transform_name("@columns"), &self.columns)?;
        res.serialize_field("image", &self.image)?;
        res.end()
    }
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct Data<T: SerializationFormat> {
    #[serde(rename = "@encoding")]
    encoding: String,
    #[serde(rename = "$text", deserialize_with = "deserialize_csv")]
    data: Vec<Vec<u32>>,
    #[serde(skip)]
    rest: PhantomData<T>,
}

pub fn deserialize_csv<'de, D>(deserializer: D) -> Result<Vec<Vec<u32>>, D::Error>
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

impl<T> Serialize for Data<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize_data(&self.data, serializer)
    }
}

impl From<Data<XmlFormat>> for Data<JsonFormat> {
    fn from(data: Data<XmlFormat>) -> Self {
        Data::<JsonFormat> {
            encoding: data.encoding,
            data: data.data,
            rest: Default::default(),
        }
    }
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct Layer<T: SerializationFormat> {
    #[serde(rename = "@id")]
    id: Option<u32>,
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
    data: Data<T>,
}

impl<T> Serialize for Layer<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = serializer.serialize_struct("layer", 7)?;
        res.serialize_field(T::transform_name("@id"), &self.id)?;
        res.serialize_field(T::transform_name("@name"), &self.name)?;
        res.serialize_field(T::transform_name("@width"), &self.width)?;
        res.serialize_field(T::transform_name("@height"), &self.height)?;
        res.serialize_field(T::transform_name("@offsetx"), &self.offsetx)?;
        res.serialize_field(T::transform_name("@offsety"), &self.offsety)?;
        res.serialize_field("data", &self.data)?;
        res.end()
    }
}

impl From<Layer<XmlFormat>> for Layer<JsonFormat> {
    fn from(layer: Layer<XmlFormat>) -> Self {
        Layer::<JsonFormat> {
            id: layer.id,
            name: layer.name,
            width: layer.width,
            height: layer.height,
            offsetx: layer.offsetx,
            offsety: layer.offsety,
            data: layer.data.into(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(bound = "T: SerializationFormat")]
struct Map<T: SerializationFormat> {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@tiledversion")]
    tiledversion: Option<String>,
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
    infinite: Option<u32>,
    #[serde(rename = "@backgroundcolor")]
    backgroundcolor: Option<String>,
    #[serde(rename = "@nextlayerid")]
    nextlayerid: Option<u32>,
    #[serde(rename = "@nextobjectid")]
    nextobjectid: u32,
    editorsettings: Option<EditorSettings<T>>,
    #[serde(rename = "tileset")]
    tilesets: Vec<TileSet<T>>,
    #[serde(rename = "layer")]
    layers: Vec<Layer<T>>,
}

impl<T> Serialize for Map<T>
where
    T: SerializationFormat,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut res = serializer.serialize_struct("map", 15)?;
        res.serialize_field(T::transform_name("@version"), &self.version)?;
        res.serialize_field(T::transform_name("@tiledversion"), &self.tiledversion)?;
        res.serialize_field(T::transform_name("@orientation"), &self.orientation)?;
        res.serialize_field(T::transform_name("@renderorder"), &self.renderorder)?;
        res.serialize_field(T::transform_name("@width"), &self.width)?;
        res.serialize_field(T::transform_name("@height"), &self.height)?;
        res.serialize_field(T::transform_name("@tilewidth"), &self.tilewidth)?;
        res.serialize_field(T::transform_name("@tileheight"), &self.tileheight)?;
        res.serialize_field(T::transform_name("@infinite"), &self.infinite)?;
        res.serialize_field(T::transform_name("@backgroundcolor"), &self.backgroundcolor)?;
        res.serialize_field(T::transform_name("@nextlayerid"), &self.nextlayerid)?;
        res.serialize_field(T::transform_name("@nextobjectid"), &self.nextobjectid)?;
        res.serialize_field("editorsettings", &self.editorsettings)?;
        res.serialize_field(T::transform_vec_name("tilesets"), &self.tilesets)?;
        res.serialize_field(T::transform_vec_name("layers"), &self.layers)?;
        res.end()
    }
}

impl From<Map<XmlFormat>> for Map<JsonFormat> {
    fn from(map: Map<XmlFormat>) -> Self {
        let editorsettings = if let Some(editorsettings) = map.editorsettings {
            Some(editorsettings.into())
        } else {
            None
        };
        let tilesets = map.tilesets.into_iter().map(|x| x.into()).collect();
        let layers = map.layers.into_iter().map(|x| x.into()).collect();
        Map::<JsonFormat> {
            version: map.version,
            tiledversion: map.tiledversion,
            orientation: map.orientation,
            renderorder: map.renderorder,
            width: map.width,
            height: map.height,
            tilewidth: map.tilewidth,
            tileheight: map.tileheight,
            infinite: map.infinite,
            backgroundcolor: map.backgroundcolor,
            nextlayerid: map.nextlayerid,
            nextobjectid: map.nextobjectid,
            editorsettings,
            tilesets,
            layers,
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let contents = fs::read_to_string(cli.file).expect("Should have been able to read the file");
    let mut map: Map<XmlFormat> = from_str(&contents).unwrap();
    if cli.command == Commands::Convert {
        let map: Map<JsonFormat> = map.into();
        let res = serde_json::to_string_pretty(&map).unwrap();
        println!("{res}");
    } else {
        let tileset = map.tilesets.iter_mut().next().expect("Needs at least one tileset");
        for layer in &mut map.layers {
            for row in &mut layer.data.data.iter_mut() {
                for cell in row.iter_mut() {
                    match cli.command {
                        Commands::Replace { find, replace } => {
                            if *cell != 0 && *cell - 1 == find {
                                *cell = replace + 1;
                            }
                        }
                        Commands::Resize { columns, .. } => {
                            if *cell >= tileset.columns {
                                *cell += (*cell - 1) / tileset.columns
                                    * (columns - tileset.columns);
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
        if let Commands::Resize { tilecount, columns } = cli.command {
            tileset.columns = columns;
            tileset.tilecount = tilecount;
        }

        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 1);
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
}
