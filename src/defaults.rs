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

    #[test]
    fn file_names_are_unique() {
        let mut names: Vec<_> = ALL.iter().map(|b| b.file_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ALL.len());
    }
}
