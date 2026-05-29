use std::env;
use std::fs;
use std::path::Path;

fn main() {
    tauri_build::build();

    // dump_index.json はリリース CI で生成される成果物。
    // 存在すれば OUT_DIR にコピーして include_bytes! で埋め込む。
    // ローカル開発環境では存在しないため空オブジェクトを置く（dump_index なしと同等）。
    // Cargo のデフォルトはパッケージ内ファイルのみ変更検知する。
    // dump_index.json はパッケージ外（../catalog/）なので明示指定が必要。
    println!("cargo:rerun-if-changed=../catalog/dump_index.json");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let src = Path::new("../catalog/dump_index.json");
    let dst = format!("{out_dir}/dump_index.json");
    if src.exists() {
        fs::copy(src, &dst).expect("failed to copy dump_index.json to OUT_DIR");
    } else {
        fs::write(&dst, b"{}").expect("failed to write empty dump_index stub to OUT_DIR");
    }
}
