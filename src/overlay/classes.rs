//! Class catalogue for the grouped class picker.
//!
//! The groups are borrowed from `config.json`'s class lists and never written as literals here
//! (PITFALLS section 10). They are a picker convenience only: nothing in the emitter reads them.
//! `jewelleryClasses` carries `Jewels` alongside Rings/Amulets/Belts so a one-item header is not
//! needed. Empty groups are omitted rather than rendered as a bare header (a store seeded before
//! `jewelleryClasses` existed parses with an empty list).

use crate::models::config::Config;

#[derive(Debug, Clone, Default)]
pub struct ClassCatalog {
    groups: Vec<(&'static str, Vec<String>)>,
}

impl ClassCatalog {
    pub fn from_config(c: &Config) -> Self {
        let groups = [
            ("WEAPONS", &c.weapon_classes),
            ("ARMOUR", &c.armour_classes),
            ("JEWELLERY", &c.jewellery_classes),
            ("FLASKS & CHARMS", &c.flask_charm_classes),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(label, v)| (label, v.clone()))
        .collect();
        ClassCatalog { groups }
    }

    pub fn groups(&self) -> &[(&'static str, Vec<String>)] {
        &self.groups
    }

    pub fn contains(&self, class: &str) -> bool {
        self.groups
            .iter()
            .any(|(_, v)| v.iter().any(|c| c == class))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed() -> Config {
        let dir = env!("CARGO_MANIFEST_DIR");
        crate::loader::load_config(
            &format!("{dir}/config.json"),
            &format!("{dir}/styles-default.json"),
        )
        .unwrap()
    }

    #[test]
    fn class_groups_are_borrowed_from_the_config_in_order() {
        let cfg = committed();
        let cat = ClassCatalog::from_config(&cfg);
        let labels: Vec<_> = cat.groups().iter().map(|g| g.0).collect();
        assert_eq!(
            labels,
            ["WEAPONS", "ARMOUR", "JEWELLERY", "FLASKS & CHARMS"]
        );
        assert_eq!(cat.groups()[0].1, cfg.weapon_classes);
        assert_eq!(cat.groups()[1].1, cfg.armour_classes);
        assert_eq!(cat.groups()[2].1, cfg.jewellery_classes);
        assert_eq!(cat.groups()[3].1, cfg.flask_charm_classes);
    }

    #[test]
    fn jewellery_is_offered_and_comes_from_the_config_not_a_literal() {
        let mut cfg = committed();
        cfg.jewellery_classes = vec!["Zzz Test Class".to_string()];
        let cat = ClassCatalog::from_config(&cfg);
        let j = cat.groups().iter().find(|g| g.0 == "JEWELLERY").unwrap();
        assert_eq!(j.1, vec!["Zzz Test Class".to_string()]);
    }

    #[test]
    fn empty_group_is_omitted() {
        let mut cfg = committed();
        cfg.jewellery_classes.clear();
        let cat = ClassCatalog::from_config(&cfg);
        assert!(cat.groups().iter().all(|g| g.0 != "JEWELLERY"));
        assert_eq!(cat.groups().len(), 3);
    }

    #[test]
    fn contains_reads_the_committed_config() {
        let cat = ClassCatalog::from_config(&committed());
        assert!(cat.contains("Jewels"));
        assert!(!cat.contains("Not A Class"));
    }

    #[test]
    fn no_class_literals_outside_tests() {
        let src = include_str!("classes.rs");
        let prod = src.split("#[cfg(test)]").next().unwrap();
        for lit in ["\"Rings\"", "\"Jewels\""] {
            assert!(!prod.contains(lit), "hardcoded {lit}");
        }
    }
}
