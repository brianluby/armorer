//! Human-readable text view of the JSON documents the CLI prints.
//!
//! Binary-private: the library and every JSON contract are untouched. The renderer takes the
//! exact `serde_json::Value` that JSON mode would print and returns a summary, or `None` when
//! the output has no text view (the caller then falls back to JSON).
//!
//! Terminal safety: text mode prints file contents and names that come from an untrusted
//! repository. Every string written here goes through [`sanitize`] (via [`Out`]), so control
//! characters and bidirectional overrides can never rewrite the reviewer's terminal. Single-line
//! contexts (`line`, table cells) also escape newlines, so a value cannot forge an extra line.

use serde_json::Value;

/// Escape every character that could alter terminal state or visually reorder a line.
///
/// Newline and tab are kept. A carriage return becomes `\r`. Every other `char::is_control()` character,
/// the Unicode bidirectional embedding/override/isolate characters U+202A..=U+202E and U+2066..=U+2069,
/// U+2028/U+2029, and the invisible format characters that make two different strings look alike or
/// reorder neighbours (U+061C, U+180E, U+200B..=U+200F, U+2060..=U+2064, U+FEFF and the tag block
/// U+E0000..=U+E007F) become a visible lowercase-hex escape: `\x1b` up to 0xFF, `\u{202e}` above.
/// Idempotent.
pub fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        // Bidirectional overrides/isolates reorder a line; U+2028/U+2029 break one in some terminals.
        let bidi = matches!(
            c,
            '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{061C}'
                | '\u{180E}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2060}'..='\u{2064}'
                | '\u{FEFF}'
                | '\u{E0000}'..='\u{E007F}'
        );
        if c == '\r' {
            out.push_str("\\r");
        } else if (c.is_control() && c != '\n' && c != '\t') || bidi {
            let code = u32::from(c);
            if code <= 0xFF {
                out.push_str(&format!("\\x{code:02x}"));
            } else {
                out.push_str(&format!("\\u{{{code:x}}}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The only way text reaches the output: every push is sanitized.
#[derive(Default)]
struct Out(String);

impl Out {
    fn push(&mut self, text: &str) {
        self.0.push_str(&sanitize(text));
    }

    /// One output line: a newline inside `text` is escaped so it cannot forge another line.
    fn line(&mut self, text: &str) {
        self.push(&text.replace('\n', "\\n"));
        self.0.push('\n');
    }

    /// Write a block verbatim (sanitized) and make sure it ends the line.
    fn block(&mut self, text: &str) {
        self.push(text);
        if !text.ends_with('\n') {
            self.0.push('\n');
        }
    }

    fn blank(&mut self) {
        self.0.push('\n');
    }

    /// Left-aligned columns; widths are measured after sanitizing.
    fn table(&mut self, indent: &str, rows: &[Vec<String>]) {
        let rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| sanitize(cell).replace('\n', "\\n"))
                    .collect()
            })
            .collect();
        let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
        let widths: Vec<usize> = (0..columns)
            .map(|i| {
                rows.iter()
                    .filter_map(|row| row.get(i))
                    .map(|cell| cell.chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for row in &rows {
            let mut line = String::from(indent);
            for (i, cell) in row.iter().enumerate() {
                line.push_str(cell);
                if i + 1 < row.len() {
                    let pad = widths[i].saturating_sub(cell.chars().count()) + 2;
                    line.push_str(&" ".repeat(pad));
                }
            }
            self.line(line.trim_end_matches(' '));
        }
    }

    /// Exactly zero trailing newlines; the caller writes the single final newline.
    fn finish(self) -> String {
        self.0.trim_end_matches('\n').to_owned()
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Render a scalar (or a compact JSON fallback for anything nested).
fn scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "none".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Produce the text view, or `None` when this output has no summary.
pub fn render(value: &Value) -> Option<String> {
    let mut out = Out::default();
    if let Some(error) = error_body(value) {
        out.line(&format!(
            "error [{}]: {}",
            text(error, "code"),
            text(error, "message")
        ));
    } else if text(value, "mode") == "bootstrap-plan" {
        bootstrap_plan(&mut out, value);
    } else if text(value, "contract") == "armorer-init-proposal" {
        init_proposal(&mut out, value);
    } else if text(value, "contract") == "armorer-upgrade-plan" {
        upgrade_plan(&mut out, value);
    } else if text(value, "contract") == "armorer-upgrade-rollback" {
        rollback_plan(&mut out, value);
    } else if value.get("outcome").is_some() && value.is_object() {
        receipt(&mut out, value);
    } else if is_catalog(value) {
        catalog(&mut out, value);
    } else {
        return None;
    }
    Some(out.finish())
}

fn error_body(value: &Value) -> Option<&Value> {
    let object = value.as_object()?;
    let body = object.get("error")?;
    (object.len() == 1 && body.get("code").is_some() && body.get("message").is_some())
        .then_some(body)
}

fn findings(out: &mut Out, value: &Value) {
    let items = array(value, "findings");
    out.blank();
    if items.is_empty() {
        out.line("Findings: none");
        return;
    }
    out.line("Findings:");
    for finding in items {
        out.line(&format!(
            "  {}: {}",
            text(finding, "code"),
            text(finding, "detail")
        ));
    }
}

fn closing(out: &mut Out, hint: &str) {
    out.blank();
    out.line(hint);
    out.line(
        "This text view is a summary; the JSON document is what the digest covers, not this text.",
    );
}

/// Say plainly when an interrupted transaction blocks applying this plan.
fn recovery_notice(out: &mut Out, plan: &Value) {
    if plan.get("recovery_required").and_then(Value::as_bool) == Some(true) {
        out.line(
            "Recovery required: an interrupted transaction must be recovered before this plan can be applied.",
        );
    }
}

fn bootstrap_plan(out: &mut Out, plan: &Value) {
    let digest = text(plan, "plan_sha256");
    out.line(&format!("Bootstrap plan {digest}"));
    out.line(&format!("Catalog: {}", text(plan, "catalog_id")));
    recovery_notice(out, plan);
    out.blank();
    out.line("Changes:");
    let changes = array(plan, "changes");
    let rows: Vec<Vec<String>> = changes
        .iter()
        .map(|change| {
            vec![
                text(change, "disposition").into(),
                if change.get("owned").and_then(Value::as_bool) == Some(true) {
                    "owned".into()
                } else {
                    "unowned".into()
                },
                text(change, "path").into(),
            ]
        })
        .collect();
    out.table("  ", &rows);
    for change in changes {
        if text(change, "disposition") == "unchanged" {
            continue;
        }
        out.blank();
        out.line(&format!(
            "Diff for {} ({}):",
            text(change, "path"),
            text(change, "disposition")
        ));
        out.block(text(change, "unified_diff"));
    }
    findings(out, plan);
    closing(
        out,
        &format!(
            "To approve exactly this plan: save the JSON output and run bootstrap apply --plan <file> --expect-plan-sha256 {digest}"
        ),
    );
}

fn init_proposal(out: &mut Out, proposal: &Value) {
    let digest = text(proposal, "proposal_sha256");
    out.line(&format!("Init proposal {digest}"));
    out.line(&format!("Repository: {}", text(proposal, "repository")));
    out.line(&format!("Toolchain: {}", text(proposal, "toolchain")));
    for file in array(proposal, "files") {
        out.blank();
        out.line(&format!("File {}:", text(file, "path")));
        out.block(text(file, "content"));
    }
    findings(out, proposal);
    closing(
        out,
        &format!(
            "To write exactly these files: init --repo-name {} --toolchain {} --write --expect-proposal-sha256 {digest}",
            text(proposal, "repository"),
            text(proposal, "toolchain")
        ),
    );
}

fn authority(value: Option<&Value>) -> String {
    match value {
        Some(authority) if authority.is_object() => format!(
            "{} (revision {})",
            text(authority, "id"),
            authority.get("revision").map_or_else(String::new, scalar)
        ),
        _ => "none".into(),
    }
}

fn upgrade_plan(out: &mut Out, plan: &Value) {
    let digest = text(plan, "plan_sha256");
    out.line(&format!("Upgrade plan {digest}"));
    out.line(&format!("Direction: {}", text(plan, "direction")));
    recovery_notice(out, plan);
    out.line(&format!(
        "Source catalog: {}",
        authority(plan.get("source_catalog"))
    ));
    out.line(&format!(
        "Target catalog: {}",
        authority(plan.get("target_catalog"))
    ));
    out.blank();
    out.line("Changes:");
    let changes = array(plan, "changes");
    let rows: Vec<Vec<String>> = changes
        .iter()
        .map(|change| vec![text(change, "decision").into(), text(change, "path").into()])
        .collect();
    out.table("  ", &rows);
    for change in changes {
        if text(change, "decision") == "unchanged" {
            continue;
        }
        let diff = text(change, "proposed_diff");
        if !diff.is_empty() {
            out.blank();
            out.line(&format!(
                "Proposed diff for {} ({}):",
                text(change, "path"),
                text(change, "decision")
            ));
            out.block(diff);
            continue;
        }
        let candidate = text(change, "candidate_diff");
        if candidate.is_empty() {
            continue;
        }
        out.blank();
        if change.get("proposed_diff").is_some_and(Value::is_string) {
            // A proposal exists and is empty (a preserved customization): apply keeps the
            // current file, so the candidate diff is information, not a pending conflict.
            out.line(&format!(
                "Candidate diff for {} ({}); the current file is kept, so this is not applied:",
                text(change, "path"),
                text(change, "decision")
            ));
        } else {
            // A blocked change has no proposal; show what disagrees so it can be resolved.
            out.line(&format!(
                "Unresolved candidate diff for {} ({}); not part of an approved proposal:",
                text(change, "path"),
                text(change, "decision")
            ));
        }
        out.block(candidate);
    }
    let pins = array(plan, "pin_changes");
    if !pins.is_empty() {
        out.blank();
        out.line("Pin changes:");
        for pin in pins {
            let version = |key: &str| {
                pin.get(key)
                    .filter(|v| v.is_object())
                    .map_or("none", |v| text(v, "version"))
            };
            out.line(&format!(
                "  {}: {} -> {}",
                text(pin, "name"),
                version("before"),
                version("after")
            ));
        }
    }
    let notes = array(plan, "compatibility");
    out.blank();
    if notes.is_empty() {
        out.line("Compatibility notes: none");
    } else {
        out.line("Compatibility notes:");
        for note in notes {
            out.line(&format!(
                "  {} [{}]: {}",
                text(note, "code"),
                text(note, "state"),
                text(note, "detail")
            ));
        }
    }
    closing(
        out,
        &format!(
            "To approve exactly this plan: save the JSON output and run upgrade apply --plan <file> --expect-plan-sha256 {digest}"
        ),
    );
}

fn rollback_plan(out: &mut Out, plan: &Value) {
    let digest = text(plan, "plan_sha256");
    let original = plan.get("original_upgrade").unwrap_or(&Value::Null);
    out.line(&format!("Rollback plan {digest}"));
    out.line("Direction: rollback");
    out.line(&format!(
        "Restores from catalog: {}",
        authority(original.get("target_catalog"))
    ));
    out.line(&format!(
        "Restores to catalog: {}",
        authority(original.get("source_catalog"))
    ));
    out.blank();
    out.line("Changes:");
    let changes = array(plan, "changes");
    let rows: Vec<Vec<String>> = changes
        .iter()
        .map(|change| {
            // A reverse image with no content removes the file; do not call that a restore.
            let action = if change.get("proposed_content").is_some_and(Value::is_null) {
                "delete"
            } else {
                "restore"
            };
            vec![action.into(), text(change, "path").into()]
        })
        .collect();
    out.table("  ", &rows);
    for change in changes {
        let diff = text(change, "unified_diff");
        if diff.is_empty() {
            continue;
        }
        out.blank();
        out.line(&format!("Proposed diff for {}:", text(change, "path")));
        out.block(diff);
    }
    let limits = array(plan, "limitations");
    out.blank();
    if limits.is_empty() {
        out.line("Limitations: none");
    } else {
        out.line("Limitations:");
        for limit in limits {
            out.line(&format!("  {}", scalar(limit)));
        }
    }
    closing(
        out,
        &format!(
            "To approve exactly this plan: save the JSON output and run upgrade rollback-apply --plan <file> --expect-plan-sha256 {digest}"
        ),
    );
}

fn receipt(out: &mut Out, receipt: &Value) {
    let Some(object) = receipt.as_object() else {
        return;
    };
    out.line(&format!(
        "outcome: {}",
        object.get("outcome").map_or_else(String::new, scalar)
    ));
    for (name, field) in object {
        if name == "outcome" {
            continue;
        }
        let rendered = match field {
            Value::Array(items) => items.iter().map(scalar).collect::<Vec<_>>().join(", "),
            other => scalar(other),
        };
        out.line(&format!("{name}: {rendered}"));
    }
}

fn is_catalog(value: &Value) -> bool {
    value.get("tools").is_some_and(Value::is_object)
        && value
            .get("workflow")
            .is_some_and(|w| w.get("repository").is_some() && w.get("commit").is_some())
}

fn catalog(out: &mut Out, catalog: &Value) {
    let workflow = catalog.get("workflow").unwrap_or(&Value::Null);
    out.line(&format!(
        "Workflow repository: {}",
        text(workflow, "repository")
    ));
    out.line(&format!("Workflow commit: {}", text(workflow, "commit")));
    out.blank();
    let mut rows = vec![vec![
        "TOOL".to_owned(),
        "VERSION".into(),
        "PLATFORMS".into(),
    ]];
    if let Some(tools) = catalog.get("tools").and_then(Value::as_object) {
        for (name, tool) in tools {
            let platforms = tool
                .get("platforms")
                .and_then(Value::as_object)
                .map(|p| p.keys().cloned().collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            rows.push(vec![name.clone(), text(tool, "version").into(), platforms]);
        }
    }
    out.table("", &rows);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitizer_escapes_controls_and_bidi_visibly() {
        let table = [
            ("plain text", "plain text"),
            ("keep\nnewline\tand tab", "keep\nnewline\tand tab"),
            ("\x1b[2K", "\\x1b[2K"),
            ("a\rb", "a\\rb"),
            ("nul\0here", "nul\\x00here"),
            ("\x7f", "\\x7f"),
            ("\u{85}", "\\x85"),
            ("\u{9f}", "\\x9f"),
            ("\u{202e}", "\\u{202e}"),
            (
                "\u{202a}\u{202b}\u{202c}\u{202d}",
                "\\u{202a}\\u{202b}\\u{202c}\\u{202d}",
            ),
            (
                "\u{2066}\u{2067}\u{2068}\u{2069}",
                "\\u{2066}\\u{2067}\\u{2068}\\u{2069}",
            ),
            ("\u{2028}\u{2029}", "\\u{2028}\\u{2029}"),
            ("caf\u{e9} \u{1f600}", "caf\u{e9} \u{1f600}"),
            ("\\x1b already", "\\x1b already"),
        ];
        for (input, expected) in table {
            assert_eq!(sanitize(input), expected, "{input:?}");
            assert_eq!(sanitize(&sanitize(input)), sanitize(input), "idempotent");
        }
    }

    #[test]
    fn invisible_format_characters_are_visible_escapes() {
        for c in [
            '\u{061C}',
            '\u{180E}',
            '\u{200B}',
            '\u{200C}',
            '\u{200D}',
            '\u{200E}',
            '\u{200F}',
            '\u{2060}',
            '\u{2064}',
            '\u{FEFF}',
            '\u{E0000}',
            '\u{E0041}',
            '\u{E007F}',
        ] {
            let escaped = sanitize(&format!("a{c}b"));
            assert_eq!(escaped, format!("a\\u{{{:x}}}b", u32::from(c)), "{c:?}");
        }
        assert_eq!(sanitize("a\u{E0080}"), "a\u{E0080}");
    }

    #[test]
    fn newline_in_a_single_line_value_cannot_forge_a_line() {
        let value = json!({
            "mode": "bootstrap-plan", "plan_sha256": "d1", "catalog_id": "c\nforged: yes",
            "findings": [{"code": "f", "detail": "x\n  fake-code: injected"}],
            "changes": [{"path": "a.yml\n  unchanged  owned    safe.yml", "disposition": "update",
                         "owned": true, "unified_diff": "-a\n+b\n"}]
        });
        let text = render(&value).unwrap();
        for forged in [
            "\nforged: yes",
            "\n  unchanged  owned    safe.yml",
            "\n  fake-code:",
        ] {
            assert!(!text.contains(forged), "{forged:?} in {text}");
        }
        assert!(text.contains("  update  owned  a.yml\\n  unchanged  owned    safe.yml\n"));
        assert!(
            text.contains("-a\n+b\n"),
            "multi-line diffs stay multi-line"
        );
        let receipt = render(&json!({"outcome": "x\ny: z", "k\nj": "v\nw: 1"})).unwrap();
        assert_eq!(receipt.lines().count(), 2, "{receipt}");
    }

    #[test]
    fn carriage_return_is_never_raw() {
        assert_eq!(sanitize("a\r\nb"), "a\\r\nb");
    }

    #[test]
    fn error_shape() {
        let value = json!({"error": {"code": "invalid", "message": "bad \x1b input"}});
        assert_eq!(render(&value).unwrap(), "error [invalid]: bad \\x1b input");
    }

    #[test]
    fn bootstrap_plan_shape() {
        let value = json!({
            "mode": "bootstrap-plan", "plan_sha256": "d1", "catalog_id": "bootstrap-v2",
            "findings": [{"code": "f-one", "detail": "first"}],
            "changes": [
                {"path": "a.yml", "disposition": "unchanged", "owned": true, "unified_diff": ""},
                {"path": "b.yml", "disposition": "conflict", "owned": false,
                 "unified_diff": "--- a/b.yml\n+++ b/b.yml\n-old\x1b\n+new\n"}
            ]
        });
        let text = render(&value).unwrap();
        assert!(text.starts_with("Bootstrap plan d1\nCatalog: bootstrap-v2\n"));
        assert!(!text.contains("Recovery required"));
        let mut pending = value.clone();
        pending["recovery_required"] = json!(true);
        assert!(
            render(&pending)
                .unwrap()
                .starts_with("Bootstrap plan d1\nCatalog: bootstrap-v2\nRecovery required: ")
        );
        assert!(text.contains("  unchanged  owned    a.yml\n"));
        assert!(text.contains("  conflict   unowned  b.yml\n"));
        assert!(text.contains("-old\\x1b\n+new\n"));
        assert!(!text.contains("Diff for a.yml"));
        assert!(text.contains("  f-one: first\n"));
        assert!(text.contains("bootstrap apply --plan <file> --expect-plan-sha256 d1\n"));
        assert!(text.ends_with("not this text."));
        assert!(!text.contains('\x1b'));
    }

    #[test]
    fn init_proposal_shape() {
        let value = json!({
            "contract": "armorer-init-proposal", "proposal_sha256": "p1",
            "repository": "o/r", "toolchain": "1.95.0",
            "files": [{"path": "armorer.toml", "content": "x = 1\n", "sha256": "s"}],
            "findings": []
        });
        let text = render(&value).unwrap();
        assert!(text.contains("Repository: o/r\nToolchain: 1.95.0\n"));
        assert!(text.contains("File armorer.toml:\nx = 1\n"));
        assert!(text.contains("Findings: none"));
        assert!(text.contains(
            "init --repo-name o/r --toolchain 1.95.0 --write --expect-proposal-sha256 p1"
        ));
    }

    #[test]
    fn upgrade_plan_shape() {
        let value = json!({
            "contract": "armorer-upgrade-plan", "plan_sha256": "u1", "direction": "upgrade",
            "source_catalog": {"id": "bootstrap-v1", "revision": 1},
            "target_catalog": {"id": "bootstrap-v2", "revision": 2},
            "changes": [
                {"path": "k", "decision": "unchanged", "proposed_diff": null},
                {"path": "w.yml", "decision": "update", "proposed_diff": "--- a\n+++ b\n+x\n"}
            ],
            "pin_changes": [{"name": "t", "before": {"version": "1.0.0"}, "after": null}],
            "compatibility": [{"state": "ok", "code": "c1", "detail": "d1"}]
        });
        let text = render(&value).unwrap();
        assert!(text.contains("Direction: upgrade"));
        assert!(!text.contains("Recovery required"));
        let mut pending = value.clone();
        pending["recovery_required"] = json!(true);
        assert!(
            render(&pending)
                .unwrap()
                .contains("Direction: upgrade\nRecovery required: ")
        );
        assert!(text.contains("Source catalog: bootstrap-v1 (revision 1)"));
        assert!(text.contains("Target catalog: bootstrap-v2 (revision 2)"));
        assert!(text.contains("  update     w.yml\n"));
        assert!(text.contains("Proposed diff for w.yml (update):\n--- a\n+++ b\n+x\n"));
        assert!(!text.contains("Proposed diff for k"));
        let mut blocked = value.clone();
        blocked["changes"] = json!([
            {"path": "c.yml", "decision": "conflict", "proposed_diff": null,
             "candidate_diff": "--- a/c.yml\n+++ b/c.yml\n-mine\x1b\n+theirs\n"}
        ]);
        let text = render(&blocked).unwrap();
        assert!(text.contains(
            "Unresolved candidate diff for c.yml (conflict); not part of an approved proposal:\n--- a/c.yml\n+++ b/c.yml\n-mine\\x1b\n+theirs\n"
        ));
        assert!(!text.contains("Proposed diff for c.yml"));
        assert!(!text.contains('\x1b'));
        let mut kept = value.clone();
        kept["changes"] = json!([
            {"path": "ci.yml", "decision": "preserve-customization", "proposed_diff": "",
             "candidate_diff": "--- a/ci.yml\n+++ b/ci.yml\n-mine\n+theirs\n"}
        ]);
        let text = render(&kept).unwrap();
        assert!(text.contains(
            "Candidate diff for ci.yml (preserve-customization); the current file is kept, so this is not applied:\n"
        ));
        assert!(!text.contains("Unresolved"));
        assert!(text.contains("  t: 1.0.0 -> none\n"));
        assert!(text.contains("  c1 [ok]: d1\n"));
        assert!(text.contains("upgrade apply --plan <file> --expect-plan-sha256 u1\n"));
    }

    #[test]
    fn rollback_plan_shape() {
        let value = json!({
            "contract": "armorer-upgrade-rollback", "plan_sha256": "r1",
            "original_upgrade": {
                "source_catalog": null,
                "target_catalog": {"id": "bootstrap-v2", "revision": 2}
            },
            "limitations": ["Local only."],
            "changes": [{"path": "w.yml", "unified_diff": "--- a/w.yml\n+++ /dev/null\n"}]
        });
        let text = render(&value).unwrap();
        assert!(text.contains("Restores from catalog: bootstrap-v2 (revision 2)"));
        assert!(text.contains("Restores to catalog: none"));
        assert!(text.contains("  restore  w.yml\n"));
        assert!(text.contains("+++ /dev/null\n"));
        let mut removal = value.clone();
        removal["changes"] = json!([
            {"path": "gone.json", "proposed_content": null, "unified_diff": "--- a/gone.json\n+++ /dev/null\n"},
            {"path": "kept.yml", "proposed_content": "x", "unified_diff": ""}
        ]);
        let text = render(&removal).unwrap();
        assert!(text.contains("  delete   gone.json\n"), "{text}");
        assert!(text.contains("  restore  kept.yml\n"), "{text}");
        assert!(text.contains("  Local only.\n"));
        assert!(text.contains("upgrade rollback-apply --plan <file> --expect-plan-sha256 r1\n"));
    }

    #[test]
    fn receipt_shape() {
        let value = json!({
            "schema_version": 1, "plan_sha256": "d", "outcome": "applied",
            "written": ["a", "b"], "configured": true
        });
        let text = render(&value).unwrap();
        assert_eq!(
            text,
            "outcome: applied\nconfigured: true\nplan_sha256: d\nschema_version: 1\nwritten: a, b"
        );
    }

    #[test]
    fn catalog_shape() {
        let value = json!({
            "schema_version": 1,
            "workflow": {"repository": "o/w", "commit": "abc"},
            "tools": {
                "zz": {"version": "2.0.0", "platforms": {"x86": {}, "arm": {}}},
                "a": {"version": "1.0.0", "platforms": {}}
            }
        });
        let text = render(&value).unwrap();
        assert!(text.starts_with("Workflow repository: o/w\nWorkflow commit: abc\n\n"));
        assert!(text.contains("TOOL  VERSION  PLATFORMS\n"));
        assert!(text.contains("a     1.0.0\n"));
        assert!(text.contains("zz    2.0.0    arm, x86"));
    }

    #[test]
    fn unknown_shapes_have_no_view() {
        assert!(render(&json!({"$schema": "x", "type": "object"})).is_none());
        assert!(render(&json!({"status": "authenticated-release-files"})).is_none());
        assert!(render(&json!({"error": {"code": "c"}})).is_none());
    }

    #[test]
    fn output_has_no_trailing_newline_for_caller_to_add() {
        let value = json!({"error": {"code": "c", "message": "m"}});
        assert!(!render(&value).unwrap().ends_with('\n'));
    }
}
