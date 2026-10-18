//! The six shipped data files, compiled into the library.
//!
//! The repo-root files stay the single source of truth: `artifact_guard`, the fixtures and the
//! tests keep reading the files, which are the same bytes. Editing a file needs a rebuild to
//! change the embedded copy. Only styles and tiers can be overridden on disk.

/// One embedded shipped file.
#[derive(Debug, PartialEq, Eq)]
pub struct Builtin {
    pub file_name: &'static str,
    pub text: &'static str,
}

impl Builtin {
    /// Name used in error messages so an embedded copy is never mistaken for a file on disk.
    pub fn label(&self) -> String {
        format!("{} (built in)", self.file_name)
    }
}

pub static STYLES_DEFAULT: Builtin = Builtin {
    file_name: "styles-default.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/styles-default.json")),
};

pub static TIERS_DEFAULT: Builtin = Builtin {
    file_name: "tiers-default.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers-default.json")),
};

pub static PRICES_FALLBACK: Builtin = Builtin {
    file_name: "prices-fallback.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prices-fallback.json")),
};

pub static TIERS_FALLBACK: Builtin = Builtin {
    file_name: "tiers-fallback.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tiers-fallback.json")),
};

/// The overlay class catalog.
pub static CONFIG: Builtin = Builtin {
    file_name: "config.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config.json")),
};

/// The sync base-type mapping.
pub static MAPPING: Builtin = Builtin {
    file_name: "mapping.json",
    text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/mapping.json")),
};

pub static ALL: [&Builtin; 6] = [
    &STYLES_DEFAULT,
    &TIERS_DEFAULT,
    &PRICES_FALLBACK,
    &TIERS_FALLBACK,
    &CONFIG,
    &MAPPING,
];

/// Look up by bare file name (no path separators accepted).
pub fn by_file_name(name: &str) -> Option<&'static Builtin> {
    ALL.iter().copied().find(|b| b.file_name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_text_equals_the_committed_file() {
        for b in ALL {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(b.file_name);
            let on_disk = std::fs::read_to_string(&path).unwrap();
            assert_eq!(b.text, on_disk, "{}", b.file_name);
        }
    }

    #[test]
    fn every_embedded_file_parses_as_json() {
        for b in ALL {
            serde_json::from_str::<serde_json::Value>(b.text)
                .unwrap_or_else(|e| panic!("{}: {e}", b.file_name));
        }
    }

    #[test]
    fn lookup_by_bare_name() {
        assert_eq!(by_file_name("tiers-fallback.json"), Some(&TIERS_FALLBACK));
        assert!(by_file_name("styles.json").is_none());
        assert!(by_file_name("sub/tiers-fallback.json").is_none());
    }

    #[test]
    fn label_marks_built_in() {
        assert_eq!(STYLES_DEFAULT.label(), "styles-default.json (built in)");
    }

    fn shipped_styles() -> crate::models::config::StyleSet {
        crate::loader::load_styles_str(STYLES_DEFAULT.text, "styles-default.json").unwrap()
    }

    #[test]
    fn shipped_styles_are_the_nine_builtins() {
        let set = shipped_styles();
        let names: Vec<&str> = set.names().collect();
        assert_eq!(
            names,
            ["S", "A", "B", "C", "D", "E", "F", "unchanged", "hidden"]
        );
        for (name, style) in set.iter() {
            let want = crate::models::config::StyleSet::builtin_id(name);
            assert_eq!(style.id.as_deref(), Some(want.as_str()), "{name}");
        }
    }

    #[test]
    fn shipped_styles_use_no_custom_sound() {
        let v: serde_json::Value = serde_json::from_str(STYLES_DEFAULT.text).unwrap();
        for (name, style) in v.as_object().unwrap() {
            let sound = &style["sound"];
            assert!(
                sound.is_null() || (sound["id"].is_number() && sound.get("custom").is_none()),
                "{name}: {sound}"
            );
        }
        assert!(!STYLES_DEFAULT.text.contains("uwu"));
    }

    #[test]
    fn shipped_styles_match_the_author_snapshot() {
        use serde_json::{Value, json};
        let v: Value = serde_json::from_str(STYLES_DEFAULT.text).unwrap();
        let star = |size: u8, color: &str| json!({"size": size, "color": color, "shape": "Star"});
        let beam = |color: &str| json!({"color": color, "temp": false});
        let want = [
            (
                "S",
                json!({"id":"builtin:S","text":[255,121,198],"border":[255,121,198],"background":[248,248,242],"font":37,"minimap":star(0,"Pink"),"beam":beam("Pink"),"sound":{"id":6,"vol":300},"action":"Show","disableDropSound":false,"enableDropSound":true}),
            ),
            (
                "A",
                json!({"id":"builtin:A","text":[189,147,249],"border":[189,147,249],"background":[248,248,242],"font":37,"minimap":star(1,"Purple"),"beam":beam("Purple"),"sound":{"id":6,"vol":300},"action":"Show","disableDropSound":false,"enableDropSound":true}),
            ),
            (
                "B",
                json!({"id":"builtin:B","text":[255,85,85],"border":[255,85,85],"background":[40,42,54],"font":37,"minimap":star(2,"Red"),"beam":beam("Red"),"sound":{"id":2,"vol":200},"action":"Show","disableDropSound":false,"enableDropSound":true}),
            ),
            (
                "C",
                json!({"id":"builtin:C","text":[139,233,253],"border":[139,233,253],"background":[40,42,54],"font":null,"minimap":star(2,"Cyan"),"beam":beam("Yellow"),"sound":{"id":2,"vol":200},"action":"Show","disableDropSound":false}),
            ),
            (
                "D",
                json!({"id":"builtin:D","text":[80,250,123],"border":[80,250,123],"background":[40,42,54],"font":null,"minimap":star(2,"Green"),"beam":beam("White"),"sound":null,"action":"Show","disableDropSound":false}),
            ),
            (
                "E",
                json!({"id":"builtin:E","text":[241,250,140],"border":[241,250,140],"background":[40,42,54],"font":23,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false}),
            ),
            (
                "F",
                json!({"id":"builtin:F","text":[248,248,242],"border":[248,248,242],"background":[40,42,54],"font":20,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false}),
            ),
            (
                "unchanged",
                json!({"id":"builtin:unchanged","text":null,"border":null,"background":null,"font":null,"minimap":null,"beam":null,"sound":null,"action":"Show","disableDropSound":false}),
            ),
            (
                "hidden",
                json!({"id":"builtin:hidden","text":null,"border":null,"background":null,"font":21,"minimap":null,"beam":null,"sound":null,"action":"Hide","disableDropSound":true}),
            ),
        ];
        assert_eq!(v.as_object().unwrap().len(), want.len());
        for (name, expected) in want {
            assert_eq!(v[name], expected, "{name}");
        }
    }

    #[test]
    fn shipped_styles_round_trip_through_render_styles() {
        assert_eq!(
            crate::loader::render_styles(&shipped_styles()),
            STYLES_DEFAULT.text
        );
    }

    #[test]
    fn file_names_are_unique() {
        let mut names: Vec<_> = ALL.iter().map(|b| b.file_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ALL.len());
    }
}
