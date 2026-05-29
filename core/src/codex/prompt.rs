//! Codex に渡すプロンプトの組み立て。

use crate::catalog::CatalogEntry;

/// システム指示 + カタログ候補 + ユーザープロンプトを組み立てる。
/// Codex は EmoteSpec(JSON) を 1 つだけ返すよう強制される（--output-schema と併用）。
pub fn build_prompt(user_prompt: &str, candidates: &[&CatalogEntry]) -> String {
    let mut s = String::new();
    s.push_str(
        "You are an expert FiveM/GTA V emote choreographer. \
Given a user's request, compose a single emote by selecting and sequencing EXISTING GTA V \
animation clips. Do NOT invent dictionary or clip names. Prefer clips from the candidate list \
below; you may use other well-known existing GTA V clips only if you are certain they exist.\n\n",
    );
    s.push_str("Rules:\n");
    s.push_str("- Output ONLY a JSON object conforming to the provided schema. No prose.\n");
    s.push_str("- `name` must be lower_snake_case ascii. `displayName` may be Japanese.\n");
    s.push_str("- `clips` is the ordered sequence to play (1+). Each needs `dict` and `clip`.\n");
    s.push_str("- Set `loop` true for continuous idle-style emotes, false for one-shot actions.\n");
    s.push_str("- Use `upperBodyOnly` true when the emote should allow walking.\n");
    s.push_str("- Set `meta.source` to \"codex\" and `meta.schemaVersion` to 1.\n");
    s.push_str("- Only add `prop`/`facial` when clearly relevant.\n\n");

    s.push_str("Candidate clips (key | dict | clip | tags):\n");
    for c in candidates {
        s.push_str(&format!(
            "- {} | {} | {} | {}\n",
            c.key,
            c.dict,
            c.clip,
            c.tags.join(",")
        ));
    }
    s.push('\n');
    s.push_str("User request:\n");
    s.push_str(user_prompt);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::types::EntryDefaults;

    fn entry() -> CatalogEntry {
        CatalogEntry {
            key: "amb_cheer".into(),
            display_name: "Cheering".into(),
            dict: "amb@world_human_cheering@male_a".into(),
            clip: "base".into(),
            category: "ambient".into(),
            tags: vec!["cheering".into()],
            defaults: EntryDefaults {
                loop_: true,
                upper_body_only: false,
                movement_type: "stationary".into(),
            },
        }
    }

    #[test]
    fn includes_user_prompt_and_candidates() {
        let e = entry();
        let p = build_prompt("乾杯して喜ぶ", &[&e]);
        assert!(p.contains("乾杯して喜ぶ"));
        assert!(p.contains("amb@world_human_cheering@male_a"));
        assert!(p.contains("Output ONLY a JSON object"));
    }
}
