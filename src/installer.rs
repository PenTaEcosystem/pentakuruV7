//! Apply a setup manifest once without overwriting account credentials or later Settings edits.
use crate::Preferences;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    schema: u32,
    installation_id: String,
    notes: bool,
    search: bool,
    spell: bool,
    sally: bool,
    ui_tars: bool,
    writer: bool,
    impress: bool,
    calc: bool,
    music: bool,
    clip: bool,
    deeptutor: bool,
}

fn apply(path: &Path, prefs: &mut Preferences) -> Result<bool, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 8192 {
        return Err("Setup manifest too large".into());
    }
    let options: Options = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if options.schema != 1
        || options.installation_id.is_empty()
        || options.installation_id.len() > 128
    {
        return Err("Unsupported setup manifest".into());
    }
    if prefs.applied_installation == options.installation_id {
        return Ok(false);
    }
    prefs.utilities.enable_notes = options.notes;
    prefs.utilities.enable_search = options.search;
    prefs.utilities.enable_spell = options.spell;
    prefs.utilities.enable_sally = options.sally;
    prefs.utilities.enable_ui_tars = options.ui_tars;
    prefs.ui_tars.enabled = options.ui_tars;
    prefs.office.writer = options.writer;
    prefs.office.impress = options.impress;
    prefs.office.calc = options.calc;
    prefs.utilities.enable_office = options.writer || options.impress || options.calc;
    prefs.integrations.music = options.music;
    prefs.integrations.clip = options.clip;
    prefs.integrations.deeptutor = options.deeptutor;
    prefs.applied_installation = options.installation_id;
    Ok(true)
}

pub fn apply_installed(prefs: &mut Preferences) -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    apply(&exe.with_file_name("installation-options.json"), prefs)
}

pub fn check_installed(output: &str) {
    let mut prefs = Preferences::default();
    let result = apply_installed(&mut prefs);
    let report = serde_json::json!({"applied":result.as_ref().copied().unwrap_or(false),
        "error":result.err(),"utilities":prefs.utilities,"writer":prefs.office.writer,
        "impress":prefs.office.impress,"calc":prefs.office.calc,
        "music":prefs.integrations.music,"clip":prefs.integrations.clip,"deeptutor":prefs.integrations.deeptutor});
    let _ = std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_selection_applies_once_and_preserves_accounts() {
        let path = std::env::temp_dir().join(format!("penta-setup-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"schema":1,"installation_id":"test-install","notes":false,"search":true,"spell":false,"sally":false,"ui_tars":false,"writer":true,"impress":false,"calc":false,"music":true,"clip":false,"deeptutor":false}"#).unwrap();
        let mut prefs = Preferences {
            note: "existing note".into(),
            ..Preferences::default()
        };
        assert!(apply(&path, &mut prefs).unwrap());
        assert!(prefs.utilities.enable_office && prefs.office.writer && prefs.integrations.music);
        assert!(!prefs.utilities.enable_sally && !prefs.integrations.deeptutor);
        assert_eq!(prefs.note, "existing note");
        prefs.integrations.music = false;
        assert!(!apply(&path, &mut prefs).unwrap());
        assert!(!prefs.integrations.music);
        std::fs::remove_file(path).unwrap();
    }
}
