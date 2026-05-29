use std::time::Instant;
use std::path::Path;
use emoteforge_core::catalog::Catalog;
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let t = Instant::now();
    let cat = Catalog::load(&root.join("catalog/catalog.json")).unwrap();
    let load_ms = t.elapsed().as_millis();
    let t2 = Instant::now();
    let cat = cat.with_dump_index(&root.join("catalog/dump_index.json")).unwrap();
    let idx_ms = t2.elapsed().as_millis();
    let t3 = Instant::now();
    let mut hits = 0;
    for _ in 0..1000 { hits += cat.search("drunk cheer dance wave coffee", 40).len(); }
    let search_us = t3.elapsed().as_micros() / 1000;
    println!("catalog.load: {load_ms} ms ({} entries)", cat.len());
    println!("with_dump_index: {idx_ms} ms (has_index={})", cat.has_dump_index());
    println!("search avg: {search_us} us/query (hits sample {hits})");
}
