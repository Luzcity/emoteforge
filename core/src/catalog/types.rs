use serde::{Deserialize, Serialize};

/// catalog.json の 1 エントリ（curated・タグ付きの既存アニメ）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub key: String,
    pub display_name: String,
    pub dict: String,
    pub clip: String,
    pub category: String,
    pub tags: Vec<String>,
    pub defaults: EntryDefaults,
}

/// エントリ採用時の既定の振る舞い。
#[derive(Debug, Clone, PartialEq)]
pub struct EntryDefaults {
    pub loop_: bool,
    pub upper_body_only: bool,
    pub movement_type: String,
}

// catalog.json は camelCase の `loop` を使うため個別対応。
impl<'de> Deserialize<'de> for EntryDefaults {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            #[serde(rename = "loop")]
            loop_: bool,
            upper_body_only: bool,
            movement_type: String,
        }
        let r = Raw::deserialize(deserializer)?;
        Ok(EntryDefaults {
            loop_: r.loop_,
            upper_body_only: r.upper_body_only,
            movement_type: r.movement_type,
        })
    }
}

impl Serialize for EntryDefaults {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EntryDefaults", 3)?;
        s.serialize_field("loop", &self.loop_)?;
        s.serialize_field("upperBodyOnly", &self.upper_body_only)?;
        s.serialize_field("movementType", &self.movement_type)?;
        s.end()
    }
}

/// catalog.json のトップレベル。
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogFile {
    pub entries: Vec<CatalogEntry>,
}
