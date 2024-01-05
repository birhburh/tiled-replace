use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};
use std::fs;

mod serialize_as_string {
    use csv::{self, Terminator};
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(d: &Vec<Vec<u32>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut res = String::new();

        for record in d {
            let mut v = Vec::new();
            let mut w = csv::WriterBuilder::new()
                .has_headers(false)
                .terminator(Terminator::Any(',' as u8))
                .from_writer(&mut v);
            w.serialize(record).map_err(serde::ser::Error::custom)?;
            drop(w);
            let mut s = String::from_utf8(v).map_err(serde::ser::Error::custom)?;
            s.push_str("\n");
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Data {
    #[serde(rename = "@encoding")]
    encoding: String,
    #[serde(rename = "$text", with = "serialize_as_string")]
    data: Vec<Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Layer {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,
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
    tileset: TileSet,
    layer: Vec<Layer>,
}

fn main() {
    let contents = fs::read_to_string("../airplane-mode/assets_src/airplane.tmx")
        .expect("Should have been able to read the file");
    let mut map: Map = from_str(&contents).unwrap();
    // dbg!(&map);
    // dbg!(&map);
    let layer = &mut map.layer[0];
    // dbg!(&layer.data);
    layer.data.data[0][0] = 11;
    // dbg!(&layer.data);
    println!("{}", to_string(&layer.data).unwrap());
}
