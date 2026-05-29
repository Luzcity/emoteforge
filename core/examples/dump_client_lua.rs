//! client.lua テンプレートを stdout に出力する補助バイナリ（構文検証/デバッグ用）。
//! 使い方: cargo run -p emoteforge_core --example dump_client_lua > client.lua

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let lua = match which.as_str() {
        "bridge" => emoteforge_core::export::lua_templates::bridge_client_lua(),
        _ => emoteforge_core::export::lua_templates::client_lua(),
    };
    print!("{lua}");
}
