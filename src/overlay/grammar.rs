//! Presentation grammar for the data-driven rule body.
//!
//! Pure: no egui types and no file IO (PITFALLS section 8), so every decision the condition and
//! action rows make is unit-testable. It answers four questions about the Phase 16 keyword table:
//!
//! * which keywords the `+ condition` / `+ action` pickers list, and in which groups;
//! * which operators a condition offers, and how they read in plain English;
//! * the slider range and add-default of each keyword;
//! * how a line's raw value tokens become a structured [`Value`] for a control, and back.
//!
//! The codec follows round-trip-or-raw (PARSE-04): [`decode`] returns a structured value only when
//! [`encode_tokens`] reproduces the input tokens exactly, otherwise [`Value::Raw`]. Displaying a
//! line can therefore never rewrite it.

use crate::filter::keywords::{
    self, COLOURS, KEYWORDS, Keyword, Kind, RARITIES, SHAPES, ValueShape,
};
use crate::filter::tokenize::Operator;

// ---------------------------------------------------------------------------------------------
// Picker groups
// ---------------------------------------------------------------------------------------------

/// Sentinel in `CONDITION_GROUPS` for the single Defences picker entry.
const DEFENCES_MARKER: &str = "@Defences";

/// Sentinel in `CONDITION_GROUPS` for the Simple Defences picker entry (right after Defences).
const SIMPLE_DEFENCES_MARKER: &str = "@SimpleDefences";

/// The three keywords the Defences entry stands for; reachable only through it.
const DEFENCE_KEYWORDS: [&str; 3] = ["BaseArmour", "BaseEvasion", "BaseEnergyShield"];

/// One entry of the condition picker: a keyword, the combined Defences card, or the Simple
/// Defences card (Any | Has | No per defence type). A rule shows at most one of the two cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionEntry {
    Keyword(&'static Keyword),
    Defences,
    SimpleDefences,
}

impl ConditionEntry {
    pub fn label(&self) -> String {
        match self {
            ConditionEntry::Defences => "Defences".to_string(),
            ConditionEntry::SimpleDefences => "Simple Defences".to_string(),
            ConditionEntry::Keyword(k) => display_name(k.name),
        }
    }

    /// Text the picker search matches against (lower case).
    pub fn search_text(&self) -> String {
        match self {
            ConditionEntry::Defences => "defences armour evasion energy shield defence".to_string(),
            ConditionEntry::SimpleDefences => {
                "simple defences armour evasion energy shield defence ar ev es".to_string()
            }
            ConditionEntry::Keyword(k) => {
                format!("{} {}", display_name(k.name), k.name).to_lowercase()
            }
        }
    }
}

const CONDITION_GROUPS: &[(&str, &[&str])] = &[
    (
        "Item",
        &[
            "Class",
            "BaseType",
            "Rarity",
            DEFENCES_MARKER,
            SIMPLE_DEFENCES_MARKER,
            "ItemLevel",
            "DropLevel",
            "Quality",
            "Width",
            "Height",
            "StackSize",
        ],
    ),
    ("Sockets", &["Sockets"]),
    (
        "Status",
        &[
            "Identified",
            "Corrupted",
            "Mirrored",
            "FracturedItem",
            "Foulborn",
            "Imbued",
            "Vestigial",
            "TwiceCorrupted",
            "CorruptedMods",
            "UnidentifiedItemTier",
        ],
    ),
    (
        "Mods",
        &[
            "HasExplicitMod",
            "HasImplicitMod",
            "HasEnchantment",
            "AnyEnchantment",
            "HasVaalUniqueMod",
            "IsVaalUnique",
        ],
    ),
    (
        "Endgame",
        &["AreaLevel", "WaystoneTier", "GemLevel", "MirageMap"],
    ),
];

#[cfg(test)]
fn resolve(names: &[&str]) -> Vec<&'static Keyword> {
    names
        .iter()
        .filter_map(|n| {
            let k = keywords::lookup(n);
            debug_assert!(k.is_some(), "grammar group names unknown keyword {n}");
            k
        })
        .collect()
}

/// Condition picker groups in UI order, with one `Defences` entry after Rarity. `Other` collects
/// every condition keyword not named in a group (the defence keywords excepted), in table order.
pub fn condition_picker_groups() -> Vec<(&'static str, Vec<ConditionEntry>)> {
    let mut out: Vec<(&'static str, Vec<ConditionEntry>)> = CONDITION_GROUPS
        .iter()
        .map(|(g, names)| {
            let entries = names
                .iter()
                .filter_map(|n| {
                    if *n == DEFENCES_MARKER {
                        Some(ConditionEntry::Defences)
                    } else if *n == SIMPLE_DEFENCES_MARKER {
                        Some(ConditionEntry::SimpleDefences)
                    } else {
                        let k = keywords::lookup(n);
                        debug_assert!(k.is_some(), "grammar group names unknown keyword {n}");
                        k.map(ConditionEntry::Keyword)
                    }
                })
                .collect();
            (*g, entries)
        })
        .collect();
    let other: Vec<ConditionEntry> = KEYWORDS
        .iter()
        .filter(|k| k.kind == Kind::Condition)
        .filter(|k| !DEFENCE_KEYWORDS.contains(&k.name))
        .filter(|k| {
            !CONDITION_GROUPS
                .iter()
                .any(|(_, names)| names.contains(&k.name))
        })
        .map(ConditionEntry::Keyword)
        .collect();
    out.push(("Other", other));
    out
}

/// Legacy keyword-only projection of [`condition_picker_groups`]: the Defences entry expands in
/// place to the three defence keywords; the Simple Defences entry expands to nothing, so each
/// defence keyword is listed once.
#[cfg(test)]
pub fn condition_groups() -> Vec<(&'static str, Vec<&'static Keyword>)> {
    condition_picker_groups()
        .into_iter()
        .map(|(g, entries)| {
            let kws = entries
                .into_iter()
                .flat_map(|e| match e {
                    ConditionEntry::Keyword(k) => vec![k],
                    ConditionEntry::Defences => resolve(&DEFENCE_KEYWORDS),
                    ConditionEntry::SimpleDefences => Vec::new(),
                })
                .collect();
            (g, kws)
        })
        .collect()
}

/// CamelCase split with spaces. `Class`, `BaseType`, `Rarity` and `Sockets` are shown as written.
pub fn display_name(name: &str) -> String {
    if matches!(name, "Class" | "BaseType" | "Rarity" | "Sockets") {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------------------------

/// One entry of the operator dropdown. `op: None` is the bare (operator-less) form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpChoice {
    pub op: Option<Operator>,
    pub label: &'static str,
}

const fn oc(op: Option<Operator>, label: &'static str) -> OpChoice {
    OpChoice { op, label }
}

/// Label for an operator form that is only offered because the file already uses it.
fn existing_label(op: Option<Operator>) -> &'static str {
    match op {
        None => "Exactly (no operator)",
        Some(Operator::Eq) => "Exactly (=)",
        Some(Operator::EqEq) => "Exactly",
        Some(Operator::Bang) => "Not (!)",
        Some(Operator::Ne) => "Not",
        Some(Operator::Lt) => "Less than",
        Some(Operator::Le) => "At most",
        Some(Operator::Gt) => "More than",
        Some(Operator::Ge) => "At least",
    }
}

fn numeric_set() -> Vec<OpChoice> {
    vec![
        oc(Some(Operator::Ge), "At least"),
        oc(Some(Operator::Le), "At most"),
        oc(Some(Operator::Gt), "More than"),
        oc(Some(Operator::Lt), "Less than"),
        oc(Some(Operator::EqEq), "Exactly"),
        oc(Some(Operator::Ne), "Not"),
    ]
}

fn ensure_current(mut v: Vec<OpChoice>, current: Option<Operator>) -> Vec<OpChoice> {
    if !v.iter().any(|c| c.op == current) {
        v.push(oc(current, existing_label(current)));
    }
    v
}

/// Operators a condition offers. The current operator is always present, so an operator already in
/// the file is shown as-is and never normalized (PARSE-04). Actions have no operator.
pub fn operator_choices(kw: &Keyword, current: Option<Operator>, list_form: bool) -> Vec<OpChoice> {
    if kw.kind != Kind::Condition {
        return Vec::new();
    }
    match kw.value {
        ValueShape::Bool | ValueShape::BoolOrName | ValueShape::OptionalBool => {
            let mut v = Vec::new();
            match current {
                Some(Operator::Eq) => v.push(oc(Some(Operator::Eq), "Is")),
                Some(Operator::EqEq) => v.push(oc(Some(Operator::EqEq), "Is")),
                _ => v.push(oc(None, "Is")),
            }
            match current {
                Some(Operator::Ne) => v.push(oc(Some(Operator::Ne), "Is not")),
                Some(Operator::Bang) => v.push(oc(Some(Operator::Bang), "Is not (!)")),
                Some(Operator::Lt | Operator::Le | Operator::Gt | Operator::Ge) => {
                    v.push(oc(current, existing_label(current)))
                }
                _ => {}
            }
            v
        }
        // A multi-rarity line cannot take an ordering operator, but a negation is valid and means
        // "none of these", so it must not be labelled "Matches" (CR-01).
        ValueShape::Rarity if list_form => match current {
            Some(Operator::Ne) => vec![oc(current, "Not any of")],
            Some(Operator::Bang) => vec![oc(current, "Not any of (!)")],
            _ => vec![oc(current, "Matches")],
        },
        ValueShape::QuotedList | ValueShape::Enum(_) => {
            let bare = match current {
                Some(Operator::Eq) => Some(Operator::Eq),
                _ => None,
            };
            let v = vec![
                oc(bare, "Matches"),
                oc(Some(Operator::EqEq), "Matches exactly"),
                oc(Some(Operator::Ne), "Not"),
            ];
            ensure_current(v, current)
        }
        _ => ensure_current(numeric_set(), current),
    }
}

/// Plain-English label of `op` for this keyword.
pub fn operator_label(kw: &Keyword, op: Option<Operator>, list_form: bool) -> &'static str {
    operator_choices(kw, op, list_form)
        .into_iter()
        .find(|c| c.op == op)
        .map_or_else(|| existing_label(op), |c| c.label)
}

// ---------------------------------------------------------------------------------------------
// Ranges and defaults
// ---------------------------------------------------------------------------------------------

/// `SetFontSize` slider bounds.
pub const FONT_SIZE_RANGE: (i64, i64) = (1, 45);
/// Alert sound id slider bounds.
pub const SOUND_ID_RANGE: (i64, i64) = (1, 16);
/// Alert sound volume slider bounds.
pub const VOLUME_RANGE: (i64, i64) = (0, 300);
/// Minimap icon size bounds (0 largest, 2 smallest).
pub const MINIMAP_SIZE_RANGE: (i64, i64) = (0, 2);

/// Slider range `(min, max, logarithmic)` for a keyword. Typing outside it stays allowed.
pub fn slider_range(name: &str) -> (i64, i64, bool) {
    match name {
        "ItemLevel" | "AreaLevel" | "DropLevel" => (1, 100, false),
        "Quality" => (0, 30, false),
        "StackSize" => (1, 5000, true),
        "BaseArmour" | "BaseEvasion" | "BaseEnergyShield" => (0, 1000, true),
        "WaystoneTier" => (1, 17, false),
        "GemLevel" => (1, 21, false),
        "UnidentifiedItemTier" => (1, 5, false),
        "Width" => (1, 2, false),
        "Height" => (1, 4, false),
        "Sockets" => (0, 6, false),
        "SetFontSize" => (FONT_SIZE_RANGE.0, FONT_SIZE_RANGE.1, false),
        _ => (0, 100, false),
    }
}

fn int_default(name: &str) -> i64 {
    match name {
        "ItemLevel" | "AreaLevel" | "DropLevel" => 65,
        "GemLevel" => 20,
        "Quality"
        | "Sockets"
        | "StackSize"
        | "WaystoneTier"
        | "BaseArmour"
        | "BaseEvasion"
        | "BaseEnergyShield"
        | "UnidentifiedItemTier" => 1,
        other => slider_range(other).0,
    }
}

/// Full line text for `+ condition` / `+ action`. `None` for list shapes and Rarity, which start
/// as pending rows: nothing is written until the first entry exists.
pub fn default_line(kw: &Keyword) -> Option<String> {
    let n = kw.name;
    let line = match kw.value {
        ValueShape::Bool => format!("{n} True"),
        ValueShape::BoolOrName => format!("{n} True"),
        ValueShape::Int => format!("{n} >= {}", int_default(n)),
        ValueShape::Sockets => format!("{n} >= 1"),
        ValueShape::Enum(list) => format!("{n} {}", list.first()?),
        ValueShape::QuotedList | ValueShape::CountThenNames | ValueShape::Rarity => return None,
        ValueShape::Rgba => {
            if n == "SetBackgroundColor" {
                format!("{n} 40 42 54 255")
            } else {
                format!("{n} 248 248 242 255")
            }
        }
        ValueShape::FontSize => format!("{n} 32"),
        ValueShape::SoundId => format!("{n} 1 300"),
        ValueShape::CustomSound => format!("{n} \"uwu.mp3\" 300"),
        ValueShape::OptionalBool | ValueShape::NoValue => n.to_string(),
        ValueShape::MinimapIcon => format!("{n} 1 White Circle"),
        ValueShape::PlayEffect => format!("{n} White"),
        ValueShape::ImportPath => return None,
    };
    Some(line)
}

// ---------------------------------------------------------------------------------------------
// Value codec
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoolOrName {
    Bool(bool),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundRef {
    Id(i64),
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Minimap {
    /// The hidden form (`-1`, optionally followed by a colour and shape), kept token-for-token.
    Hidden(Vec<String>),
    Icon {
        size: i64,
        colour: String,
        shape: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    Colour { colour: String, temp: bool },
}

/// A line's value tokens in a shape a control can edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Ints(Vec<i64>),
    Bool {
        value: bool,
    },
    Rarity(Vec<&'static str>),
    List(Vec<String>),
    CountNames {
        count: Option<i64>,
        names: Vec<String>,
    },
    Enum(Vec<String>),
    Sockets {
        count: Option<i64>,
        colours: String,
    },
    BoolOrName(BoolOrName),
    Rgba {
        rgba: [u8; 4],
        had_alpha: bool,
    },
    FontSize(i64),
    Sound {
        id: SoundRef,
        volume: Option<i64>,
    },
    CustomSound {
        file: String,
        volume: Option<i64>,
    },
    Minimap(Minimap),
    Effect(Effect),
    OptionalBool(Option<bool>),
    NoValue,
    /// Tokens the structured controls cannot reproduce exactly; edited as raw text.
    Raw(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    Empty,
    BadText,
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Empty => f.write_str("Pick at least one value"),
            EncodeError::BadText => {
                f.write_str("Values cannot contain quotes or control characters")
            }
        }
    }
}

impl std::error::Error for EncodeError {}

fn bad_char(c: char) -> bool {
    c == '"' || (c as u32) < 0x20 || c == '\u{7f}'
}

fn bad_text(s: &str) -> bool {
    s.chars().any(bad_char)
}

/// A bare word (enum value, named sound, socket colours): no whitespace on top of `bad_text`.
fn bad_word(s: &str) -> bool {
    bad_text(s) || s.chars().any(char::is_whitespace)
}

fn quote(s: &str) -> String {
    format!("\"{s}\"")
}

/// Strip exactly one pair of quotes; `None` when the token is not a well-formed quoted name.
fn unquote(t: &str) -> Option<&str> {
    let inner = t.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.is_empty() && !bad_text(inner)).then_some(inner)
}

fn parse_int(t: &str) -> Option<i64> {
    t.parse::<i64>().ok()
}

/// Token-wise encoding of a value, without validating it.
pub fn encode_tokens(v: &Value) -> Vec<String> {
    match v {
        Value::Ints(xs) => xs.iter().map(i64::to_string).collect(),
        Value::Bool { value } => vec![bool_word(*value).to_string()],
        Value::Rarity(rs) => rs.iter().map(|r| (*r).to_string()).collect(),
        Value::List(names) => names.iter().map(|n| quote(n)).collect(),
        Value::CountNames { count, names } => count
            .iter()
            .map(i64::to_string)
            .chain(names.iter().map(|n| quote(n)))
            .collect(),
        Value::Enum(ws) => ws.clone(),
        Value::Sockets { count, colours } => {
            vec![format!(
                "{}{}",
                count.map(|c| c.to_string()).unwrap_or_default(),
                colours
            )]
        }
        Value::BoolOrName(BoolOrName::Bool(b)) => vec![bool_word(*b).to_string()],
        Value::BoolOrName(BoolOrName::Name(n)) => vec![quote(n)],
        Value::Rgba { rgba, had_alpha } => {
            let mut t: Vec<String> = rgba[..3].iter().map(u8::to_string).collect();
            if *had_alpha || rgba[3] != 255 {
                t.push(rgba[3].to_string());
            }
            t
        }
        Value::FontSize(n) => vec![n.to_string()],
        Value::Sound { id, volume } => {
            let mut t = vec![match id {
                SoundRef::Id(i) => i.to_string(),
                SoundRef::Named(n) => n.clone(),
            }];
            t.extend(volume.map(|x| x.to_string()));
            t
        }
        Value::CustomSound { file, volume } => {
            let mut t = vec![quote(file)];
            t.extend(volume.map(|x| x.to_string()));
            t
        }
        Value::Minimap(Minimap::Hidden(t)) => t.clone(),
        Value::Minimap(Minimap::Icon {
            size,
            colour,
            shape,
        }) => vec![size.to_string(), colour.clone(), shape.clone()],
        Value::Effect(Effect::None) => vec!["None".to_string()],
        Value::Effect(Effect::Colour { colour, temp }) => {
            let mut t = vec![colour.clone()];
            if *temp {
                t.push("Temp".to_string());
            }
            t
        }
        Value::OptionalBool(b) => b.iter().map(|b| bool_word(*b).to_string()).collect(),
        Value::NoValue => Vec::new(),
        Value::Raw(t) => t.clone(),
    }
}

fn bool_word(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// Space-joined value text, refusing an empty value and any text that could break out of its
/// quotes or start a new line (T-20-05).
pub fn encode(v: &Value) -> Result<String, EncodeError> {
    let empty = match v {
        Value::Ints(x) => x.is_empty(),
        Value::Rarity(x) => x.is_empty(),
        Value::List(x) => x.is_empty(),
        Value::Enum(x) => x.is_empty(),
        Value::Raw(x) => x.is_empty(),
        Value::CountNames { names, .. } => names.is_empty(),
        Value::Sockets { count, colours } => count.is_none() && colours.is_empty(),
        _ => false,
    };
    if empty {
        return Err(EncodeError::Empty);
    }
    let bad = match v {
        Value::List(x) => x.iter().any(|n| bad_text(n)),
        Value::CountNames { names, .. } => names.iter().any(|n| bad_text(n)),
        Value::CustomSound { file, .. } => bad_text(file),
        Value::BoolOrName(BoolOrName::Name(n)) => bad_text(n),
        Value::Enum(x) => x.iter().any(|n| bad_word(n)),
        Value::Sockets { colours, .. } => bad_word(colours),
        Value::Sound {
            id: SoundRef::Named(n),
            ..
        } => bad_word(n),
        _ => false,
    };
    if bad {
        return Err(EncodeError::BadText);
    }
    Ok(encode_tokens(v).join(" "))
}

fn candidate(kw: &Keyword, t: &[&str]) -> Option<Value> {
    let n = t.len();
    match kw.value {
        ValueShape::NoValue => (n == 0).then_some(Value::NoValue),
        ValueShape::Bool => match t {
            ["True"] => Some(Value::Bool { value: true }),
            ["False"] => Some(Value::Bool { value: false }),
            _ => None,
        },
        ValueShape::Int => {
            let xs: Option<Vec<i64>> = t.iter().map(|x| parse_int(x)).collect();
            xs.filter(|x| !x.is_empty()).map(Value::Ints)
        }
        ValueShape::Rarity => {
            let rs: Option<Vec<&'static str>> = t
                .iter()
                .map(|x| RARITIES.iter().find(|r| **r == *x).copied())
                .collect();
            rs.filter(|r| !r.is_empty()).map(Value::Rarity)
        }
        ValueShape::QuotedList => {
            let names: Option<Vec<String>> =
                t.iter().map(|x| unquote(x).map(str::to_string)).collect();
            names.filter(|x| !x.is_empty()).map(Value::List)
        }
        ValueShape::CountThenNames => {
            let (count, rest) = match t.first().and_then(|x| parse_int(x)) {
                Some(c) => (Some(c), &t[1..]),
                None => (None, t),
            };
            let names: Option<Vec<String>> = rest
                .iter()
                .map(|x| unquote(x).map(str::to_string))
                .collect();
            names
                .filter(|x| !x.is_empty())
                .map(|names| Value::CountNames { count, names })
        }
        ValueShape::Enum(list) => {
            if n == 0 || !t.iter().all(|x| list.contains(x)) {
                return None;
            }
            Some(Value::Enum(t.iter().map(|x| x.to_string()).collect()))
        }
        ValueShape::Sockets => {
            let [w] = t else { return None };
            let digits: String = w.chars().take_while(char::is_ascii_digit).collect();
            let colours = &w[digits.len()..];
            if !colours.chars().all(|c| "RGBWAD".contains(c)) {
                return None;
            }
            let count = if digits.is_empty() {
                None
            } else {
                Some(parse_int(&digits)?)
            };
            Some(Value::Sockets {
                count,
                colours: colours.to_string(),
            })
        }
        ValueShape::BoolOrName => match t {
            ["True"] => Some(Value::BoolOrName(BoolOrName::Bool(true))),
            ["False"] => Some(Value::BoolOrName(BoolOrName::Bool(false))),
            [x] => unquote(x).map(|s| Value::BoolOrName(BoolOrName::Name(s.to_string()))),
            _ => None,
        },
        ValueShape::OptionalBool => match t {
            [] => Some(Value::OptionalBool(None)),
            ["True"] => Some(Value::OptionalBool(Some(true))),
            ["False"] => Some(Value::OptionalBool(Some(false))),
            _ => None,
        },
        ValueShape::Rgba => {
            if n != 3 && n != 4 {
                return None;
            }
            let mut rgba = [255u8; 4];
            for (slot, x) in rgba.iter_mut().zip(t) {
                *slot = x.parse::<u8>().ok()?;
            }
            Some(Value::Rgba {
                rgba,
                had_alpha: n == 4,
            })
        }
        ValueShape::FontSize => match t {
            [x] => parse_int(x).map(Value::FontSize),
            _ => None,
        },
        ValueShape::SoundId => {
            if n != 1 && n != 2 {
                return None;
            }
            let id = match parse_int(t[0]) {
                Some(i) => SoundRef::Id(i),
                None if !t[0].is_empty() && t[0].chars().all(|c| c.is_ascii_alphanumeric()) => {
                    SoundRef::Named(t[0].to_string())
                }
                None => return None,
            };
            let volume = match t.get(1) {
                Some(x) => Some(parse_int(x)?),
                None => None,
            };
            Some(Value::Sound { id, volume })
        }
        ValueShape::CustomSound => {
            if n != 1 && n != 2 {
                return None;
            }
            let file = unquote(t[0])?.to_string();
            let volume = match t.get(1) {
                Some(x) => Some(parse_int(x)?),
                None => None,
            };
            Some(Value::CustomSound { file, volume })
        }
        ValueShape::MinimapIcon => {
            if t.first() == Some(&"-1") {
                return Some(Value::Minimap(Minimap::Hidden(
                    t.iter().map(|x| x.to_string()).collect(),
                )));
            }
            let [size, colour, shape] = t else {
                return None;
            };
            if !COLOURS.contains(colour) || !SHAPES.contains(shape) {
                return None;
            }
            Some(Value::Minimap(Minimap::Icon {
                size: parse_int(size)?,
                colour: colour.to_string(),
                shape: shape.to_string(),
            }))
        }
        ValueShape::PlayEffect => match t {
            ["None"] => Some(Value::Effect(Effect::None)),
            [c] if COLOURS.contains(c) => Some(Value::Effect(Effect::Colour {
                colour: c.to_string(),
                temp: false,
            })),
            [c, "Temp"] if COLOURS.contains(c) => Some(Value::Effect(Effect::Colour {
                colour: c.to_string(),
                temp: true,
            })),
            _ => None,
        },
        ValueShape::ImportPath => None,
    }
}

/// Decode a line's raw value tokens (quoted tokens keep their quotes). Structured only when it
/// re-encodes to exactly the same tokens; otherwise [`Value::Raw`].
pub fn decode(kw: &Keyword, tokens: &[&str]) -> Value {
    if let Some(v) = candidate(kw, tokens) {
        let enc = encode_tokens(&v);
        if enc.len() == tokens.len() && enc.iter().zip(tokens).all(|(a, b)| a == b) {
            return v;
        }
    }
    Value::Raw(tokens.iter().map(|t| t.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::keywords::lookup;

    fn kw(n: &str) -> &'static Keyword {
        lookup(n).unwrap()
    }

    fn names(g: &[(&'static str, Vec<&'static Keyword>)]) -> Vec<&'static str> {
        g.iter()
            .flat_map(|(_, v)| v.iter().map(|k| k.name))
            .collect()
    }

    #[test]
    fn every_condition_is_reachable_exactly_once() {
        let all = names(&condition_groups());
        for k in KEYWORDS.iter().filter(|k| k.kind == Kind::Condition) {
            assert_eq!(
                all.iter().filter(|n| **n == k.name).count(),
                1,
                "{}",
                k.name
            );
        }
        assert_eq!(
            all.len(),
            KEYWORDS
                .iter()
                .filter(|k| k.kind == Kind::Condition)
                .count()
        );
        for (_, ns) in CONDITION_GROUPS {
            for n in ns
                .iter()
                .filter(|n| **n != DEFENCES_MARKER && **n != SIMPLE_DEFENCES_MARKER)
            {
                assert!(lookup(n).is_some(), "group names unknown keyword {n}");
            }
        }
        let labels: Vec<_> = condition_groups().iter().map(|g| g.0).collect();
        assert_eq!(
            labels,
            ["Item", "Sockets", "Status", "Mods", "Endgame", "Other"]
        );
    }

    #[test]
    fn picker_has_one_defences_entry_after_rarity() {
        let groups = condition_picker_groups();
        let labels: Vec<_> = groups.iter().map(|g| g.0).collect();
        assert_eq!(
            labels,
            ["Item", "Sockets", "Status", "Mods", "Endgame", "Other"]
        );
        let item: Vec<String> = groups[0].1.iter().map(|e| e.label()).collect();
        assert_eq!(
            item,
            [
                "Class",
                "BaseType",
                "Rarity",
                "Defences",
                "Simple Defences",
                "Item Level",
                "Drop Level",
                "Quality",
                "Width",
                "Height",
                "Stack Size"
            ]
        );
        for (_, es) in &groups {
            for e in es {
                if let ConditionEntry::Keyword(k) = e {
                    assert!(!DEFENCE_KEYWORDS.contains(&k.name), "{}", k.name);
                }
            }
        }
        assert!(ConditionEntry::Defences.search_text().contains("armour"));
        let at = item.iter().position(|l| l == "Rarity").unwrap();
        assert_eq!(item[at + 1], "Defences");
        assert_eq!(item[at + 2], "Simple Defences");
        let simple = ConditionEntry::SimpleDefences.search_text();
        assert!(simple.contains("simple") && simple.contains("evasion"));
    }

    #[test]
    fn legacy_projection_lists_each_defence_keyword_once_inside_item() {
        let groups = condition_groups();
        let item: Vec<_> = groups[0].1.iter().map(|k| k.name).collect();
        let at = item.iter().position(|n| *n == "BaseArmour").unwrap();
        assert_eq!(
            &item[at..at + 3],
            ["BaseArmour", "BaseEvasion", "BaseEnergyShield"]
        );
        assert_eq!(item[at - 1], "Rarity");
        let all = names(&groups);
        for n in DEFENCE_KEYWORDS {
            assert_eq!(all.iter().filter(|x| **x == n).count(), 1);
        }
    }

    #[test]
    fn display_names() {
        assert_eq!(
            display_name("UnidentifiedItemTier"),
            "Unidentified Item Tier"
        );
        assert_eq!(display_name("BaseEnergyShield"), "Base Energy Shield");
        assert_eq!(display_name("BaseType"), "BaseType");
        for n in ["Class", "Rarity", "Sockets"] {
            assert_eq!(display_name(n), n);
        }
    }

    fn ops(c: &[OpChoice]) -> Vec<(Option<Operator>, &'static str)> {
        c.iter().map(|c| (c.op, c.label)).collect()
    }

    #[test]
    fn int_operator_set_and_existing_forms() {
        let k = kw("ItemLevel");
        let base = vec![
            (Some(Operator::Ge), "At least"),
            (Some(Operator::Le), "At most"),
            (Some(Operator::Gt), "More than"),
            (Some(Operator::Lt), "Less than"),
            (Some(Operator::EqEq), "Exactly"),
            (Some(Operator::Ne), "Not"),
        ];
        assert_eq!(ops(&operator_choices(k, Some(Operator::Ge), false)), base);
        let mut with_none = base.clone();
        with_none.push((None, "Exactly (no operator)"));
        assert_eq!(ops(&operator_choices(k, None, false)), with_none);
        let mut with_eq = base.clone();
        with_eq.push((Some(Operator::Eq), "Exactly (=)"));
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::Eq), false)),
            with_eq
        );
        let mut with_bang = base;
        with_bang.push((Some(Operator::Bang), "Not (!)"));
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::Bang), false)),
            with_bang
        );
    }

    #[test]
    fn list_operator_set() {
        let k = kw("Class");
        let want = vec![
            (None, "Matches"),
            (Some(Operator::EqEq), "Matches exactly"),
            (Some(Operator::Ne), "Not"),
        ];
        assert_eq!(ops(&operator_choices(k, None, true)), want);
        let eq = ops(&operator_choices(k, Some(Operator::Eq), true));
        assert_eq!(eq[0], (Some(Operator::Eq), "Matches"));
        assert_eq!(eq.len(), 3);
        assert_eq!(
            operator_label(k, Some(Operator::EqEq), true),
            "Matches exactly"
        );
    }

    #[test]
    fn bool_operator_set() {
        let k = kw("Corrupted");
        assert_eq!(ops(&operator_choices(k, None, false)), [(None, "Is")]);
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::EqEq), false)),
            [(Some(Operator::EqEq), "Is")]
        );
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::Ne), false)),
            [(None, "Is"), (Some(Operator::Ne), "Is not")]
        );
    }

    #[test]
    fn rarity_operator_sets() {
        let k = kw("Rarity");
        assert_eq!(operator_choices(k, Some(Operator::Ge), false).len(), 6);
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::EqEq), true)),
            [(Some(Operator::EqEq), "Matches")]
        );
        assert_eq!(
            ops(&operator_choices(k, Some(Operator::Ne), true)),
            [(Some(Operator::Ne), "Not any of")]
        );
        assert_eq!(
            operator_label(k, Some(Operator::Bang), true),
            "Not any of (!)"
        );
    }

    #[test]
    fn actions_have_no_operators() {
        assert!(operator_choices(kw("SetTextColor"), None, false).is_empty());
    }

    #[test]
    fn slider_ranges() {
        assert_eq!(slider_range("ItemLevel"), (1, 100, false));
        assert_eq!(slider_range("Quality"), (0, 30, false));
        assert_eq!(slider_range("StackSize"), (1, 5000, true));
        assert_eq!(slider_range("BaseArmour"), (0, 1000, true));
        assert_eq!(slider_range("WaystoneTier"), (1, 17, false));
        assert_eq!(slider_range("UnidentifiedItemTier"), (1, 5, false));
        assert_eq!(slider_range("Width"), (1, 2, false));
        assert_eq!(slider_range("Height"), (1, 4, false));
        assert_eq!(slider_range("Sockets"), (0, 6, false));
        assert_eq!(slider_range("Nonsense"), (0, 100, false));
    }

    #[test]
    fn default_lines() {
        let cases = [
            ("ItemLevel", Some("ItemLevel >= 65")),
            ("AreaLevel", Some("AreaLevel >= 65")),
            ("Quality", Some("Quality >= 1")),
            ("GemLevel", Some("GemLevel >= 20")),
            ("Corrupted", Some("Corrupted True")),
            ("BaseType", None),
            ("Class", None),
            ("Rarity", None),
            ("HasExplicitMod", None),
            ("Sockets", Some("Sockets >= 1")),
            ("SetTextColor", Some("SetTextColor 248 248 242 255")),
            (
                "SetBackgroundColor",
                Some("SetBackgroundColor 40 42 54 255"),
            ),
            ("SetFontSize", Some("SetFontSize 32")),
            ("MinimapIcon", Some("MinimapIcon 1 White Circle")),
            ("PlayEffect", Some("PlayEffect White")),
            ("PlayAlertSound", Some("PlayAlertSound 1 300")),
            ("CustomAlertSound", Some("CustomAlertSound \"uwu.mp3\" 300")),
            ("DisableDropSound", Some("DisableDropSound")),
        ];
        for (n, want) in cases {
            assert_eq!(default_line(kw(n)).as_deref(), want, "{n}");
        }
    }

    #[test]
    fn every_numeric_default_is_inside_its_slider() {
        fn within(n: i64, (lo, hi): (i64, i64), what: &str) {
            assert!(lo <= n && n <= hi, "{what}: {n} outside {lo}..={hi}");
        }
        for k in KEYWORDS {
            let Some(line) = default_line(k) else {
                continue;
            };
            let toks: Vec<&str> = line.split_whitespace().collect();
            let num = |s: &str| s.parse::<i64>().unwrap();
            match k.value {
                ValueShape::Int | ValueShape::Sockets => {
                    let (lo, hi, _) = slider_range(k.name);
                    within(num(toks[2]), (lo, hi), k.name);
                }
                ValueShape::FontSize => within(num(toks[1]), FONT_SIZE_RANGE, k.name),
                ValueShape::SoundId => {
                    within(num(toks[1]), SOUND_ID_RANGE, k.name);
                    within(num(toks[2]), VOLUME_RANGE, k.name);
                }
                ValueShape::CustomSound => within(num(toks[2]), VOLUME_RANGE, k.name),
                ValueShape::MinimapIcon => within(num(toks[1]), MINIMAP_SIZE_RANGE, k.name),
                _ => {}
            }
        }
    }

    // ---- codec ----

    // No keyword uses the Enum or BoolOrName shapes today; keep their codec arms covered.
    static TEST_ENUM: Keyword = Keyword {
        name: "TestEnum",
        kind: Kind::Condition,
        ops: keywords::Ops::Compare,
        value: ValueShape::Enum(&["Shaper", "Elder"]),
    };
    static TEST_BOOL_OR_NAME: Keyword = Keyword {
        name: "TestBoolOrName",
        kind: Kind::Condition,
        ops: keywords::Ops::Compare,
        value: ValueShape::BoolOrName,
    };

    fn d(k: &str, t: &[&str]) -> Value {
        decode(kw(k), t)
    }

    #[test]
    fn decode_shapes() {
        assert_eq!(d("ItemLevel", &["70"]), Value::Ints(vec![70]));
        assert_eq!(d("ItemLevel", &["60", "65"]), Value::Ints(vec![60, 65]));
        assert_eq!(d("Corrupted", &["True"]), Value::Bool { value: true });
        assert!(matches!(d("Corrupted", &["true"]), Value::Raw(_)));
        assert_eq!(d("Rarity", &["Magic"]), Value::Rarity(vec!["Magic"]));
        assert_eq!(
            d("Rarity", &["Normal", "Magic", "Rare"]),
            Value::Rarity(vec!["Normal", "Magic", "Rare"])
        );
        assert!(matches!(d("Rarity", &["Epic"]), Value::Raw(_)));
        assert_eq!(
            d("Class", &["\"Rings\"", "\"Amulets\""]),
            Value::List(vec!["Rings".into(), "Amulets".into()])
        );
        assert!(matches!(d("Class", &["Rings"]), Value::Raw(_)));
        assert_eq!(
            d("HasExplicitMod", &["1", "\"Tyrannical\""]),
            Value::CountNames {
                count: Some(1),
                names: vec!["Tyrannical".into()]
            }
        );
        assert_eq!(
            d("HasExplicitMod", &["\"A\""]),
            Value::CountNames {
                count: None,
                names: vec!["A".into()]
            }
        );
        assert_eq!(
            d("Sockets", &["5GGG"]),
            Value::Sockets {
                count: Some(5),
                colours: "GGG".into()
            }
        );
        assert_eq!(
            d("Sockets", &["3"]),
            Value::Sockets {
                count: Some(3),
                colours: String::new()
            }
        );
        assert_eq!(
            d("Sockets", &["RGB"]),
            Value::Sockets {
                count: None,
                colours: "RGB".into()
            }
        );
        assert!(matches!(d("Sockets", &["5X"]), Value::Raw(_)));
        assert_eq!(
            d("SetTextColor", &["255", "121", "198"]),
            Value::Rgba {
                rgba: [255, 121, 198, 255],
                had_alpha: false
            }
        );
        assert_eq!(
            d("SetTextColor", &["255", "121", "198", "200"]),
            Value::Rgba {
                rgba: [255, 121, 198, 200],
                had_alpha: true
            }
        );
        assert_eq!(d("SetFontSize", &["37"]), Value::FontSize(37));
        assert_eq!(
            d("PlayAlertSound", &["6", "300"]),
            Value::Sound {
                id: SoundRef::Id(6),
                volume: Some(300)
            }
        );
        assert_eq!(
            d("PlayAlertSound", &["1"]),
            Value::Sound {
                id: SoundRef::Id(1),
                volume: None
            }
        );
        assert_eq!(
            d("PlayAlertSound", &["ShAlchemy"]),
            Value::Sound {
                id: SoundRef::Named("ShAlchemy".into()),
                volume: None
            }
        );
        assert_eq!(
            d("CustomAlertSound", &["\"uwu.mp3\"", "300"]),
            Value::CustomSound {
                file: "uwu.mp3".into(),
                volume: Some(300)
            }
        );
        assert_eq!(
            d("MinimapIcon", &["0", "Pink", "Circle"]),
            Value::Minimap(Minimap::Icon {
                size: 0,
                colour: "Pink".into(),
                shape: "Circle".into()
            })
        );
        assert_eq!(
            d("MinimapIcon", &["-1"]),
            Value::Minimap(Minimap::Hidden(vec!["-1".into()]))
        );
        assert_eq!(
            d("PlayEffect", &["Red", "Temp"]),
            Value::Effect(Effect::Colour {
                colour: "Red".into(),
                temp: true
            })
        );
        assert_eq!(d("PlayEffect", &["None"]), Value::Effect(Effect::None));
        assert_eq!(d("DisableDropSound", &[]), Value::OptionalBool(None));
        assert_eq!(
            d("DisableDropSound", &["True"]),
            Value::OptionalBool(Some(true))
        );
        assert_eq!(
            decode(&TEST_BOOL_OR_NAME, &["True"]),
            Value::BoolOrName(BoolOrName::Bool(true))
        );
        assert_eq!(
            decode(&TEST_ENUM, &["Shaper"]),
            Value::Enum(vec!["Shaper".into()])
        );
        assert!(matches!(decode(&TEST_ENUM, &["Bogus"]), Value::Raw(_)));
    }

    #[test]
    fn rgba_three_token_form_survives() {
        let v = d("SetTextColor", &["1", "2", "3"]);
        assert_eq!(encode(&v).unwrap(), "1 2 3");
        let v = d("SetTextColor", &["1", "2", "3", "255"]);
        assert_eq!(encode(&v).unwrap(), "1 2 3 255");
    }

    #[test]
    fn non_canonical_numbers_stay_raw() {
        assert!(matches!(d("ItemLevel", &["070"]), Value::Raw(_)));
        assert!(matches!(d("ItemLevel", &["+5"]), Value::Raw(_)));
    }

    #[test]
    fn encode_refuses_bad_values() {
        assert_eq!(encode(&Value::List(vec![])), Err(EncodeError::Empty));
        assert_eq!(
            encode(&Value::List(vec!["A\"B".into()])),
            Err(EncodeError::BadText)
        );
        assert_eq!(
            encode(&Value::List(vec!["A\tB".into()])),
            Err(EncodeError::BadText)
        );
        assert_eq!(
            encode(&Value::CustomSound {
                file: "a\nb".into(),
                volume: None
            }),
            Err(EncodeError::BadText)
        );
        assert_eq!(
            encode(&Value::CountNames {
                count: Some(1),
                names: vec![]
            }),
            Err(EncodeError::Empty)
        );
        assert_eq!(
            encode(&Value::List(vec!["Rings".into(), "Belts".into()])).unwrap(),
            "\"Rings\" \"Belts\""
        );
        assert_eq!(EncodeError::Empty.to_string(), "Pick at least one value");
    }

    #[test]
    fn filterblade_lines_roundtrip_or_raw() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/parser/filterblade.filter"
        ));
        let doc = crate::filter::parse(src.as_bytes()).unwrap();
        let (mut total, mut raw) = (0usize, 0usize);
        for b in doc.blocks() {
            for bl in b.body() {
                let (Some(k), Some(t)) = (bl.keyword, &bl.tokens) else {
                    continue;
                };
                if !matches!(k.kind, Kind::Condition | Kind::Action) {
                    continue;
                }
                let text = doc.lines()[bl.line].text();
                let g = if b.is_enabled() {
                    text
                } else {
                    crate::filter::commented_inner(text)
                };
                let toks: Vec<&str> = t.values.iter().map(|r| &g[r.clone()]).collect();
                let v = decode(k, &toks);
                let enc = encode_tokens(&v);
                assert_eq!(enc, toks, "{text}");
                total += 1;
                if matches!(v, Value::Raw(_)) {
                    raw += 1;
                }
            }
        }
        assert!(total > 1000, "corpus too small: {total}");
        assert!(raw * 20 < total, "raw fallbacks {raw} of {total}");
    }
}
