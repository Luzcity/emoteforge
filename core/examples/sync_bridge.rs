//! 同梱の Preview Bridge リソース（`fivem/emoteforge_bridge/`）を Lua テンプレートから
//! 再生成する。テンプレート（`export::lua_templates`）を唯一の正とし、手書き管理による
//! ドリフトを防ぐ。`cargo run -p emoteforge-core --example sync_bridge` で実行する。
//!
//! 検証は `preview::tests::bundled_bridge_matches_templates` が CI で行う。

use std::path::Path;

use emoteforge_core::export::lua_templates::{
    bridge_client_lua, BRIDGE_FXMANIFEST, BRIDGE_SERVER_LUA,
};

fn main() -> std::io::Result<()> {
    // CARGO_MANIFEST_DIR = <repo>/core なので 1 つ上がリポジトリルート。
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fivem/emoteforge_bridge");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("client.lua"), bridge_client_lua())?;
    std::fs::write(dir.join("server.lua"), BRIDGE_SERVER_LUA)?;
    std::fs::write(dir.join("fxmanifest.lua"), BRIDGE_FXMANIFEST)?;
    println!("synced bridge resource to {}", dir.display());
    Ok(())
}
