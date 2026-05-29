//! アニメカタログの読込・検索・存在検証。

pub mod types;

pub use types::{CatalogEntry, EntryDefaults};

use std::collections::{HashMap, HashSet};
use std::path::Path;
use types::CatalogFile;

/// カタログ本体。curated エントリと（任意で）全 dump の存在索引を保持。
#[derive(Debug, Default)]
pub struct Catalog {
    entries: Vec<CatalogEntry>,
    /// dict -> clip 集合。存在検証用（任意ロード）。
    dump_index: Option<HashMap<String, HashSet<String>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("failed to read catalog: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse catalog json: {0}")]
    Parse(#[from] serde_json::Error),
}

impl Catalog {
    /// curated catalog.json を読み込む。
    pub fn load(catalog_path: &Path) -> Result<Self, CatalogError> {
        let text = std::fs::read_to_string(catalog_path)?;
        let file: CatalogFile = serde_json::from_str(&text)?;
        Ok(Catalog {
            entries: file.entries,
            dump_index: None,
        })
    }

    /// 任意で dump_index.json（{dict:[clips]}）を読み込み、存在検証を厳密化する。
    pub fn with_dump_index(mut self, index_path: &Path) -> Result<Self, CatalogError> {
        if !index_path.exists() {
            return Ok(self);
        }
        let text = std::fs::read_to_string(index_path)?;
        let raw: HashMap<String, Vec<String>> = serde_json::from_str(&text)?;
        let map = raw
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect::<HashSet<_>>()))
            .collect();
        self.dump_index = Some(map);
        Ok(self)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn has_dump_index(&self) -> bool {
        self.dump_index.is_some()
    }

    /// dict/clip が実在するか。dump_index があればそれを正とし、無ければ curated を見る。
    pub fn contains(&self, dict: &str, clip: &str) -> bool {
        if let Some(idx) = &self.dump_index {
            return idx.get(dict).is_some_and(|clips| clips.contains(clip));
        }
        self.entries
            .iter()
            .any(|e| e.dict == dict && e.clip == clip)
    }

    /// 指定 dict に存在するクリップ候補（dump_index 優先）。補修候補の提示に使う。
    pub fn clips_of(&self, dict: &str) -> Vec<String> {
        if let Some(idx) = &self.dump_index {
            if let Some(clips) = idx.get(dict) {
                let mut v: Vec<String> = clips.iter().cloned().collect();
                v.sort();
                return v;
            }
        }
        self.entries
            .iter()
            .filter(|e| e.dict == dict)
            .map(|e| e.clip.clone())
            .collect()
    }

    /// クエリ語に対しタグ/名前/dict をスコアリングし、上位 limit 件を返す。
    pub fn search(&self, query: &str, limit: usize) -> Vec<&CatalogEntry> {
        let terms = tokenize(query);
        if terms.is_empty() {
            return self.entries.iter().take(limit).collect();
        }
        let mut scored: Vec<(i32, &CatalogEntry)> = self
            .entries
            .iter()
            .map(|e| (score(e, &terms), e))
            .filter(|(s, _)| *s > 0)
            .collect();
        // スコア降順、同点は key 昇順で安定化。
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.key.cmp(&b.1.key)));
        scored.into_iter().take(limit).map(|(_, e)| e).collect()
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    /// カテゴリ横断で多様なエントリを最大 n 件サンプリングする。
    /// 検索がヒットしない（例: 日本語プロンプト）場合の候補底上げに使う。
    pub fn diverse_sample(&self, n: usize) -> Vec<&CatalogEntry> {
        use std::collections::BTreeMap;
        // カテゴリごとに分類し、ラウンドロビンで取り出す。
        let mut by_cat: BTreeMap<&str, Vec<&CatalogEntry>> = BTreeMap::new();
        for e in &self.entries {
            by_cat.entry(e.category.as_str()).or_default().push(e);
        }
        let mut out = Vec::with_capacity(n);
        let mut round = 0;
        loop {
            let mut added_any = false;
            for entries in by_cat.values() {
                if let Some(e) = entries.get(round) {
                    out.push(*e);
                    added_any = true;
                    if out.len() >= n {
                        return out;
                    }
                }
            }
            if !added_any {
                break;
            }
            round += 1;
        }
        out
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_string())
        .collect()
}

fn score(entry: &CatalogEntry, terms: &[String]) -> i32 {
    let mut score = 0;
    for term in terms {
        if entry.tags.iter().any(|t| t == term) {
            score += 3; // タグ完全一致は強い
        } else if entry.tags.iter().any(|t| t.contains(term.as_str())) {
            score += 1;
        }
        if entry.display_name.to_lowercase().contains(term.as_str()) {
            score += 1;
        }
        if entry.category == *term {
            score += 1;
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn catalog_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/catalog.json")
    }

    fn load() -> Catalog {
        Catalog::load(&catalog_path()).expect("catalog loads")
    }

    #[test]
    fn loads_nonempty_catalog() {
        let c = load();
        assert!(
            c.len() > 100,
            "expected a substantial catalog, got {}",
            c.len()
        );
    }

    #[test]
    fn contains_checks_curated_when_no_dump_index() {
        let c = load();
        let first = &c.entries()[0];
        assert!(c.contains(&first.dict, &first.clip));
        assert!(!c.contains("totally_made_up_dict", "nope"));
    }

    #[test]
    fn search_finds_relevant_entries() {
        let c = load();
        let hits = c.search("coffee", 10);
        assert!(!hits.is_empty(), "coffee should match ambient entries");
        assert!(hits
            .iter()
            .all(|e| e.tags.iter().any(|t| t.contains("coffee"))
                || e.display_name.to_lowercase().contains("coffee")));
    }

    #[test]
    fn search_respects_limit() {
        let c = load();
        let hits = c.search("idle", 5);
        assert!(hits.len() <= 5);
    }

    #[test]
    fn dump_index_makes_contains_authoritative() {
        let idx_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/dump_index.json");
        if !idx_path.exists() {
            return; // 生成物が無い環境ではスキップ
        }
        let c = load().with_dump_index(&idx_path).unwrap();
        assert!(c.has_dump_index());
        // dump にある既知の組
        assert!(c.contains("amb@world_human_cheering@male_a", "base"));
        assert!(!c.contains("amb@world_human_cheering@male_a", "no_such_clip"));
    }
}
