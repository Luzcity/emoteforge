//! Bridge resource ファイルをリポジトリの fivem/ に書き出す補助。
fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fivem");
    let out = emoteforge_core::preview::install_bridge(&dir).unwrap();
    println!("wrote bridge to {}", out.display());
}
