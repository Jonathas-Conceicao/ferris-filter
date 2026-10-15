//! Grouped rules: one overlay rule written as several consecutive blocks (Simple Defences).
//!
//! A filter block can only match a *product set* of defence base types: per type (Armour,
//! Evasion, Energy Shield) it requires the type (`BaseArmour > 0`), forbids it (`BaseEvasion 0`)
//! or leaves it free (no line). The Simple Defences card lets the user pick any set of the seven
//! base-type combinations; a pick no single block can express is written as the minimum number of
//! blocks whose union is exactly the pick ([`cover`]). Those blocks are identical apart from their
//! defence lines and each carries `# ff:group <id> <k>/<n>`; [`analyze`] finds them again on load
//! so the overlay can show the group as one rule.
//!
//! Pure document logic: no overlay types, no IO. Every mutation goes through the validated
//! [`Document`] edit API, so an `Err` leaves the document as it was.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::block::{Block, BlockId, DisabledState};
use super::edit::EditError;
use super::keywords::Kind;
use super::section;
use super::tokenize::Operator;
use super::{Document, commented_inner};

/// The metadata key of a group line: `# ff:group <id> <k>/<n>`.
pub const GROUP_KEY: &str = "group";

/// The three defence keywords, in card order.
pub const DEFENCE_NAMES: [&str; 3] = ["BaseArmour", "BaseEvasion", "BaseEnergyShield"];

/// One defence type's requirement in a block: no line, a "has this defence" line, or a "has none
/// of it" line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefenceSeg {
    Any,
    Has,
    No,
}

/// What one block requires of the three defence types, in [`DEFENCE_NAMES`] order.
pub type Constraint = [DefenceSeg; 3];

/// No defence line at all.
pub const ALL_ANY: Constraint = [DefenceSeg::Any; 3];

/// The seven base-type combinations, in chip order: label, tooltip, membership per type.
pub const COMBOS: [(&str, &str, [bool; 3]); 7] = [
    ("Full AR", "Armour bases", [true, false, false]),
    ("Full EV", "Evasion bases", [false, true, false]),
    ("Full ES", "Energy Shield bases", [false, false, true]),
    ("AR/EV", "Armour and Evasion bases", [true, true, false]),
    (
        "AR/ES",
        "Armour and Energy Shield bases",
        [true, false, true],
    ),
    (
        "EV/ES",
        "Evasion and Energy Shield bases",
        [false, true, true],
    ),
    (
        "AR/EV/ES",
        "Armour, Evasion and Energy Shield bases",
        [true, true, true],
    ),
];

/// Every combination selected.
pub const ALL_COMBOS: u8 = 0x7F;

/// The segment a defence line stands for when it is zero based, `None` otherwise: exactly one
/// integer value, and `0` with no operator, `=` or `==` (No), or `> 0` or `>= 1` (Has).
pub fn zero_based_seg(op: Option<Operator>, values: &[&str]) -> Option<DefenceSeg> {
    let [v] = values else {
        return None;
    };
    let v: i64 = v.parse().ok()?;
    match (op, v) {
        (None | Some(Operator::Eq | Operator::EqEq), 0) => Some(DefenceSeg::No),
        (Some(Operator::Gt), 0) | (Some(Operator::Ge), 1) => Some(DefenceSeg::Has),
        _ => None,
    }
}

/// A combination matches when it has every Has type and none of the No types.
pub fn combo_matches(c: Constraint, combo: [bool; 3]) -> bool {
    c.iter().zip(combo).all(|(s, member)| match s {
        DefenceSeg::Any => true,
        DefenceSeg::Has => member,
        DefenceSeg::No => !member,
    })
}

/// The combinations a constraint matches, as a bit set over [`COMBOS`] (bit i = `COMBOS[i]`).
pub fn mask_of(c: Constraint) -> u8 {
    COMBOS
        .iter()
        .enumerate()
        .filter(|(_, (_, _, combo))| combo_matches(c, *combo))
        .fold(0u8, |m, (i, _)| m | (1 << i))
}

fn lines_of(c: Constraint) -> usize {
    c.iter().filter(|s| **s != DefenceSeg::Any).count()
}

/// All 27 constraints in a fixed order.
fn all_constraints() -> Vec<Constraint> {
    let segs = [DefenceSeg::Any, DefenceSeg::Has, DefenceSeg::No];
    let mut out = Vec::with_capacity(27);
    for a in segs {
        for e in segs {
            for s in segs {
                out.push([a, e, s]);
            }
        }
    }
    out
}

fn compute_cover(sel: u8) -> Vec<Constraint> {
    let sel = sel & ALL_COMBOS;
    if sel == 0 {
        return vec![ALL_ANY];
    }
    // Candidates: every constraint matching something, and nothing outside the selection. Sorted
    // by line count first, so the first cover found among equals is the one with fewer lines.
    let mut cands: Vec<(Constraint, u8, usize)> = all_constraints()
        .into_iter()
        .map(|c| (c, mask_of(c), lines_of(c)))
        .filter(|(_, m, _)| *m != 0 && m & !sel == 0)
        .collect();
    cands.sort_by_key(|(_, _, l)| *l);
    for k in 1..=cands.len() {
        let mut best: Option<(usize, Vec<usize>)> = None;
        let mut pick: Vec<usize> = (0..k).collect();
        loop {
            let union = pick.iter().fold(0u8, |m, &i| m | cands[i].1);
            if union == sel {
                let lines: usize = pick.iter().map(|&i| cands[i].2).sum();
                if best.as_ref().is_none_or(|(b, _)| lines < *b) {
                    best = Some((lines, pick.clone()));
                }
            }
            // Next k-combination of 0..cands.len(), lexicographic.
            let n = cands.len();
            let Some(i) = (0..k).rev().find(|&i| pick[i] != i + n - k) else {
                break;
            };
            pick[i] += 1;
            for j in i + 1..k {
                pick[j] = pick[j - 1] + 1;
            }
        }
        if let Some((_, pick)) = best {
            let mut parts: Vec<Constraint> = pick.iter().map(|&i| cands[i].0).collect();
            // Stable part order: by the first chip each part covers, then by line count.
            parts.sort_by_key(|c| (mask_of(*c).trailing_zeros(), lines_of(*c)));
            return parts;
        }
    }
    vec![ALL_ANY]
}

/// The blocks a selection is written as: the minimum number of constraints whose matched
/// combinations together are exactly `sel` (a bit set over [`COMBOS`]), fewer lines on ties.
/// No selection and every combination both give one block without defence lines.
pub fn cover(sel: u8) -> &'static [Constraint] {
    static TABLE: OnceLock<Vec<Vec<Constraint>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| (0..=ALL_COMBOS).map(compute_cover).collect());
    &table[usize::from(sel & ALL_COMBOS)]
}

/// The canonical line of each non-Any type: `BaseArmour > 0` (Has), `BaseEvasion 0` (No).
pub fn canonical_texts(c: Constraint) -> Vec<String> {
    DEFENCE_NAMES
        .iter()
        .zip(c)
        .filter_map(|(name, s)| match s {
            DefenceSeg::Any => None,
            DefenceSeg::Has => Some(format!("{name} > 0")),
            DefenceSeg::No => Some(format!("{name} 0")),
        })
        .collect()
}

/// A parsed `# ff:group` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMeta {
    pub gid: String,
    pub k: usize,
    pub n: usize,
}

fn gid_ok(gid: &str) -> bool {
    (1..=16).contains(&gid.len())
        && gid
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

/// Read `<id> <k>/<n>` (1 <= k <= n, n >= 2, id 1-16 lowercase alphanumerics).
pub fn parse_meta(value: &str) -> Option<GroupMeta> {
    let mut it = value.split_whitespace();
    let (gid, kn) = (it.next()?, it.next()?);
    if it.next().is_some() || !gid_ok(gid) {
        return None;
    }
    let (k, n) = kn.split_once('/')?;
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(k) || !all_digits(n) {
        return None;
    }
    let (k, n): (usize, usize) = (k.parse().ok()?, n.parse().ok()?);
    (n >= 2 && (1..=n).contains(&k)).then(|| GroupMeta {
        gid: gid.to_string(),
        k,
        n,
    })
}

/// The `# ff:group` value of part `k` of `n`.
pub fn meta_value(gid: &str, k: usize, n: usize) -> String {
    format!("{gid} {k}/{n}")
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn id_from_u64(v: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let len = 6 + (v >> 60) as usize % 3;
    let mut x = v;
    (0..len)
        .map(|_| {
            let d = DIGITS[(x % 36) as usize] as char;
            x /= 36;
            d
        })
        .collect()
}

/// Every group id written anywhere in the document (parsable or not).
fn gids_in(doc: &Document) -> HashSet<String> {
    doc.lines()
        .iter()
        .filter_map(|l| match l.ff_marker() {
            Some((k, Some(v))) if k == GROUP_KEY => v.split_whitespace().next().map(str::to_string),
            _ => None,
        })
        .collect()
}

/// A new random group id (6-8 lowercase alphanumerics) not used by any group line in `doc`.
pub fn fresh_gid(doc: &Document) -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let taken = gids_in(doc);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut state = nanos
        ^ RandomState::new().build_hasher().finish()
        ^ COUNTER.fetch_add(0x9E37_79B9, Ordering::Relaxed);
    loop {
        let candidate = id_from_u64(splitmix64(&mut state));
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
}

/// The grammar text of a block line: past the `#` (and one space) of a disabled block.
fn grammar_text<'a>(b: &Block, text: &'a str) -> &'a str {
    if b.is_enabled() {
        text
    } else {
        commented_inner(text)
    }
}

/// Doc line indices of the block's defence condition lines, in file order.
pub fn defence_lines(b: &Block) -> Vec<usize> {
    b.body()
        .iter()
        .filter(|bl| {
            bl.keyword.is_some_and(|k| {
                k.kind == Kind::Condition
                    && DEFENCE_NAMES.iter().any(|n| n.eq_ignore_ascii_case(k.name))
            })
        })
        .map(|bl| bl.line)
        .collect()
}

/// What the block's defence lines require, when every one of them is zero based and no type has
/// two lines. A block without defence lines gives [`ALL_ANY`].
pub fn block_constraint(doc: &Document, b: &Block) -> Option<Constraint> {
    let mut c = ALL_ANY;
    let mut seen = [false; 3];
    for bl in b.body() {
        let Some(kw) = bl.keyword else { continue };
        if kw.kind != Kind::Condition {
            continue;
        }
        let Some(t) = DEFENCE_NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(kw.name))
        else {
            continue;
        };
        let tok = bl.tokens.as_ref()?;
        let text = grammar_text(b, doc.lines().get(bl.line)?.text());
        let values: Vec<&str> = tok
            .values
            .iter()
            .filter_map(|r| text.get(r.clone()))
            .collect();
        let seg = zero_based_seg(tok.operator.as_ref().map(|(_, o)| *o), &values)?;
        if seen[t] {
            return None;
        }
        seen[t] = true;
        c[t] = seg;
    }
    Some(c)
}

/// The block's defence lines as written (grammar text, indentation dropped), for re-inserting
/// into another copy of the rule.
fn defence_texts(doc: &Document, b: &Block) -> Vec<String> {
    defence_lines(b)
        .into_iter()
        .map(|i| {
            grammar_text(b, doc.lines()[i].text())
                .trim_start_matches([' ', '\t'])
                .to_string()
        })
        .collect()
}

/// The block's `# ff:group` lines (key matches, any value).
fn group_lines(doc: &Document, b: &Block) -> Vec<usize> {
    b.leading()
        .filter(|&i| {
            doc.lines()[i]
                .ff_marker()
                .is_some_and(|(k, _)| k == GROUP_KEY)
        })
        .collect()
}

/// The span text of a block with its group line(s) and defence lines left out: what every part of
/// a group must share.
fn skeleton<'a>(doc: &'a Document, b: &Block) -> Vec<&'a str> {
    let skip: HashSet<usize> = group_lines(doc, b)
        .into_iter()
        .chain(defence_lines(b))
        .collect();
    b.span()
        .filter(|i| !skip.contains(i))
        .map(|i| doc.lines()[i].text())
        .collect()
}

/// Where a block stands among the document's groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// First block of a valid group; holds every member (itself first), in file order.
    Leader(Vec<BlockId>),
    /// A later block of a valid group, with its leader.
    Follower(BlockId),
    /// Carries a group line, but the group is not valid; the block is an ordinary rule.
    Broken,
}

/// The groups of a document, by block id. Blocks without a group line are absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupIndex {
    roles: HashMap<BlockId, Role>,
}

impl GroupIndex {
    pub fn role(&self, id: BlockId) -> Option<&Role> {
        self.roles.get(&id)
    }

    /// Every block of the rule `id` leads (just `[id]` when it leads no group).
    pub fn members(&self, id: BlockId) -> Vec<BlockId> {
        match self.roles.get(&id) {
            Some(Role::Leader(m)) => m.clone(),
            _ => vec![id],
        }
    }

    /// The rule a block belongs to (its group's leader, or itself).
    pub fn rule_of(&self, id: BlockId) -> BlockId {
        match self.roles.get(&id) {
            Some(Role::Follower(l)) => *l,
            _ => id,
        }
    }

    pub fn is_follower(&self, id: BlockId) -> bool {
        matches!(self.roles.get(&id), Some(Role::Follower(_)))
    }

    pub fn is_broken(&self, id: BlockId) -> bool {
        matches!(self.roles.get(&id), Some(Role::Broken))
    }

    pub fn followers(&self) -> HashSet<BlockId> {
        self.roles
            .iter()
            .filter(|(_, r)| matches!(r, Role::Follower(_)))
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Find every group of the document. A group is valid when its blocks are consecutive (only blank
/// lines between them), carry one `# ff:group <id> <k>/<n>` line each with k = 1..n in order and
/// the same n, the id is used by no other block, none is in the economy section or an invalid
/// commented-out block, every defence line is zero based, and the blocks are identical apart from
/// their defence lines and k. Every block of an invalid group is [`Role::Broken`].
pub fn analyze(doc: &Document) -> GroupIndex {
    let span = section::find(doc).ok().flatten();
    let in_economy = |h: usize| span.is_some_and(|s| h > s.begin && h < s.end);
    let blocks: Vec<&Block> = doc.blocks().collect();
    let mut roles = HashMap::new();
    // gid -> (block position, parsed meta), in file order.
    let mut by_gid: HashMap<String, Vec<(usize, GroupMeta)>> = HashMap::new();
    for (pos, b) in blocks.iter().enumerate() {
        if in_economy(b.header_line()) {
            continue;
        }
        let lines = group_lines(doc, b);
        if lines.is_empty() {
            continue;
        }
        let meta = match lines.as_slice() {
            [one] => doc.lines()[*one]
                .ff_marker()
                .and_then(|(_, v)| v)
                .and_then(parse_meta),
            _ => None,
        };
        match meta {
            Some(m) => by_gid.entry(m.gid.clone()).or_default().push((pos, m)),
            None => {
                roles.insert(b.id(), Role::Broken);
            }
        }
    }
    for list in by_gid.values() {
        let valid = group_valid(doc, &blocks, list);
        let ids: Vec<BlockId> = list.iter().map(|(p, _)| blocks[*p].id()).collect();
        if valid {
            roles.insert(ids[0], Role::Leader(ids.clone()));
            for id in &ids[1..] {
                roles.insert(*id, Role::Follower(ids[0]));
            }
        } else {
            for id in ids {
                roles.insert(id, Role::Broken);
            }
        }
    }
    GroupIndex { roles }
}

fn group_valid(doc: &Document, blocks: &[&Block], list: &[(usize, GroupMeta)]) -> bool {
    let n = list[0].1.n;
    if list.len() != n || n < 2 {
        return false;
    }
    let p0 = list[0].0;
    let in_order = list
        .iter()
        .enumerate()
        .all(|(i, (p, m))| *p == p0 + i && m.k == i + 1 && m.n == n);
    if !in_order {
        return false;
    }
    let members: Vec<&Block> = list.iter().map(|(p, _)| blocks[*p]).collect();
    let usable = members.iter().all(|b| {
        !matches!(b.disabled_state(), Some(DisabledState::Invalid(_)))
            && block_constraint(doc, b).is_some()
    });
    if !usable {
        return false;
    }
    let blank_gaps = members.windows(2).all(|w| {
        (w[0].span().end..w[1].span().start).all(|i| doc.lines()[i].text().trim().is_empty())
    });
    if !blank_gaps {
        return false;
    }
    let first = skeleton(doc, members[0]);
    members[1..].iter().all(|b| skeleton(doc, b) == first)
}

/// The combinations a rule matches: the union over its blocks. `None` when a block's defence
/// lines are not zero based.
pub fn selection(doc: &Document, members: &[BlockId]) -> Option<u8> {
    members.iter().try_fold(0u8, |m, id| {
        let b = doc.block(*id)?;
        Some(m | mask_of(block_constraint(doc, b)?))
    })
}

/// Replace the block's defence lines with `texts` (grammar text, no indentation) in one commit:
/// the new lines go where the first old one was, else after the last condition, else after the
/// header.
fn set_defence_texts(doc: &mut Document, id: BlockId, texts: &[String]) -> Result<(), EditError> {
    let b = doc.block(id).ok_or(EditError::UnknownBlock)?;
    let remove = defence_lines(b);
    if remove.is_empty() && texts.is_empty() {
        return Ok(());
    }
    let after = match remove.first() {
        Some(first) => first - 1,
        None => b
            .body()
            .iter()
            .filter(|bl| bl.keyword.is_some_and(|k| k.kind == Kind::Condition))
            .map(|bl| bl.line)
            .next_back()
            .unwrap_or(b.header_line()),
    };
    doc.splice_body(id, &remove, after, texts)
}

/// Give the block the defence lines of `c`; a block that already reads as `c` keeps its bytes.
pub fn set_constraint(doc: &mut Document, id: BlockId, c: Constraint) -> Result<(), EditError> {
    let b = doc.block(id).ok_or(EditError::UnknownBlock)?;
    if block_constraint(doc, b) == Some(c) {
        return Ok(());
    }
    set_defence_texts(doc, id, &canonical_texts(c))
}

/// How a generated part gets its defence lines.
enum PartDefence<'a> {
    /// These exact texts (a follower keeping its own lines).
    Texts(&'a [String]),
    Constraint(Constraint),
}

/// The span text of a copy of block `source` with other defence lines and group line (`None`
/// removes it), built on a scratch document so indentation and the disabled form come out the
/// way every other edit writes them.
fn part_span(
    doc: &Document,
    source: BlockId,
    defence: PartDefence<'_>,
    meta: Option<&str>,
    name: Option<&str>,
) -> Result<Vec<String>, EditError> {
    let b = doc.block(source).ok_or(EditError::UnknownBlock)?;
    let mut text = String::new();
    for i in b.span() {
        text.push_str(doc.lines()[i].text());
        text.push('\n');
    }
    let mut scratch = super::parse(text.as_bytes()).map_err(|_| EditError::NotOneBlock)?;
    let id = scratch
        .blocks()
        .next()
        .map(Block::id)
        .ok_or(EditError::NotOneBlock)?;
    match defence {
        PartDefence::Texts(t) => set_defence_texts(&mut scratch, id, t)?,
        PartDefence::Constraint(c) => set_constraint(&mut scratch, id, c)?,
    }
    match meta {
        Some(v) => scratch.set_meta(id, GROUP_KEY, v)?,
        None => scratch.remove_meta(id, GROUP_KEY)?,
    }
    if let Some(n) = name {
        scratch.set_meta(id, "name", n)?;
    }
    let b = scratch.block(id).ok_or(EditError::NotOneBlock)?;
    Ok(b.span()
        .map(|i| scratch.lines()[i].text().to_string())
        .collect())
}

fn gid_of(doc: &Document, id: BlockId) -> Option<String> {
    doc.meta_get(id, GROUP_KEY)
        .and_then(parse_meta)
        .map(|m| m.gid)
}

/// After an edit of the leader, rewrite the other blocks of the group as copies of it, each
/// keeping its own defence lines (as written) and its `k/n`.
pub fn resync(doc: &mut Document, members: &[BlockId]) -> Result<(), EditError> {
    let Some((&leader, followers)) = members.split_first() else {
        return Ok(());
    };
    if followers.is_empty() {
        return Ok(());
    }
    let gid = gid_of(doc, leader).ok_or(EditError::NotOneBlock)?;
    let n = members.len();
    let mut spans = Vec::with_capacity(followers.len());
    for (i, f) in followers.iter().enumerate() {
        let fb = doc.block(*f).ok_or(EditError::UnknownBlock)?;
        let texts = defence_texts(doc, fb);
        let meta = meta_value(&gid, i + 2, n);
        spans.push(part_span(
            doc,
            leader,
            PartDefence::Texts(&texts),
            Some(&meta),
            None,
        )?);
    }
    doc.replace_blocks_after(leader, followers, spans)?;
    Ok(())
}

/// Write the rule (`members`, leader first) as the [`cover`] of `sel`: the leader gets the first
/// part's lines, the rest become copies of it right after. The group line is added (with a fresh
/// id, or the group's own) past one block and removed at one block. Returns the new members.
pub fn set_selection(
    doc: &mut Document,
    members: &[BlockId],
    sel: u8,
) -> Result<Vec<BlockId>, EditError> {
    let Some((&leader, followers)) = members.split_first() else {
        return Err(EditError::UnknownBlock);
    };
    let parts = cover(sel);
    let n = parts.len();
    set_constraint(doc, leader, parts[0])?;
    if n == 1 {
        doc.remove_meta(leader, GROUP_KEY)?;
        if !followers.is_empty() {
            doc.replace_blocks_after(leader, followers, Vec::new())?;
        }
        return Ok(vec![leader]);
    }
    let gid = if followers.is_empty() {
        None
    } else {
        gid_of(doc, leader)
    }
    .unwrap_or_else(|| fresh_gid(doc));
    doc.set_meta(leader, GROUP_KEY, &meta_value(&gid, 1, n))?;
    let mut spans = Vec::with_capacity(n - 1);
    for (k, c) in parts.iter().enumerate().skip(1) {
        let meta = meta_value(&gid, k + 1, n);
        spans.push(part_span(
            doc,
            leader,
            PartDefence::Constraint(*c),
            Some(&meta),
            None,
        )?);
    }
    let mut out = vec![leader];
    out.extend(doc.replace_blocks_after(leader, followers, spans)?);
    Ok(out)
}

/// Insert a copy of the rule (`members`, leader first) right after it. A group copy gets a fresh
/// group id; a single block's copy loses any group line. `name` replaces the copy's name. Returns
/// the copy's blocks.
pub fn duplicate(
    doc: &mut Document,
    members: &[BlockId],
    name: Option<&str>,
) -> Result<Vec<BlockId>, EditError> {
    let last = *members.last().ok_or(EditError::UnknownBlock)?;
    let n = members.len();
    let gid = (n > 1).then(|| fresh_gid(doc));
    let mut spans = Vec::with_capacity(n);
    for (k, m) in members.iter().enumerate() {
        let b = doc.block(*m).ok_or(EditError::UnknownBlock)?;
        let texts = defence_texts(doc, b);
        let meta = gid.as_ref().map(|g| meta_value(g, k + 1, n));
        spans.push(part_span(
            doc,
            *m,
            PartDefence::Texts(&texts),
            meta.as_deref(),
            name,
        )?);
    }
    doc.replace_blocks_after(last, &[], spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::parse;

    fn p(s: &str) -> Document {
        parse(s.as_bytes()).unwrap()
    }
    fn ids(d: &Document) -> Vec<BlockId> {
        d.blocks().map(|b| b.id()).collect()
    }

    /// Minimum number of allowed sets whose union is `sel`, by breadth-first search over unions:
    /// an independent check of `cover`'s minimality.
    fn bfs_min(sel: u8) -> usize {
        let sets: Vec<u8> = all_constraints()
            .into_iter()
            .map(mask_of)
            .filter(|m| *m != 0 && m & !sel == 0)
            .collect();
        let mut dist = [usize::MAX; 128];
        dist[0] = 0;
        let mut queue = std::collections::VecDeque::from([0u8]);
        while let Some(u) = queue.pop_front() {
            for s in &sets {
                let v = (u | s) as usize;
                if dist[v] == usize::MAX {
                    dist[v] = dist[u as usize] + 1;
                    queue.push_back(v as u8);
                }
            }
        }
        dist[sel as usize]
    }

    #[test]
    fn every_selection_is_covered_exactly_and_minimally() {
        let mut max = 0;
        for sel in 1..=ALL_COMBOS {
            let parts = cover(sel);
            let union = parts.iter().fold(0u8, |m, c| m | mask_of(*c));
            assert_eq!(union, sel, "selection {sel:07b}");
            for c in parts {
                assert_eq!(mask_of(*c) & !sel, 0, "{sel:07b}: {c:?} matches too much");
                assert_ne!(mask_of(*c), 0);
            }
            assert_eq!(parts.len(), bfs_min(sel), "selection {sel:07b} not minimal");
            max = max.max(parts.len());
        }
        assert_eq!(max, 4);
        assert_eq!(cover(0), &[ALL_ANY]);
        assert_eq!(cover(ALL_COMBOS), &[ALL_ANY]);
    }

    #[test]
    fn cover_examples() {
        use DefenceSeg::*;
        // "No EV" = Full AR, Full ES, AR/ES: one block.
        assert_eq!(cover(0b001_0101), &[[Any, No, Any]]);
        // The three pure bases: three blocks.
        assert_eq!(cover(0b000_0111).len(), 3);
        // The three pure bases plus the triple hybrid: four.
        assert_eq!(cover(0b100_0111).len(), 4);
        // Full AR alone: "no EV, no ES" (items without any defence are not a combination).
        assert_eq!(cover(0b000_0001), &[[Any, No, No]]);
        // Ties prefer fewer lines: "has AR" is one line, not a product of three.
        assert_eq!(cover(0b101_1001), &[[Has, Any, Any]]);
    }

    #[test]
    fn meta_parsing() {
        assert_eq!(
            parse_meta("ab12 2/3"),
            Some(GroupMeta {
                gid: "ab12".into(),
                k: 2,
                n: 3
            })
        );
        for bad in [
            "ab 1/1", "ab 0/2", "ab 3/2", "AB 1/2", "ab 1/2 x", "ab", "ab 1-2", "a_b 1/2",
            "ab +1/2",
        ] {
            assert_eq!(parse_meta(bad), None, "{bad}");
        }
    }

    const GROUP: &str = "# ff:name Pure\n# ff:group abc 1/2\nShow\n  Rarity Rare\n  BaseArmour > 0\n  BaseEvasion 0\n  BaseEnergyShield 0\n  SetFontSize 30\n\n# ff:name Pure\n# ff:group abc 2/2\nShow\n  Rarity Rare\n  BaseArmour 0\n  BaseEvasion > 0\n  BaseEnergyShield 0\n  SetFontSize 30\n";

    #[test]
    fn analyze_finds_a_valid_group() {
        let d = p(GROUP);
        let g = analyze(&d);
        let i = ids(&d);
        assert_eq!(g.role(i[0]), Some(&Role::Leader(vec![i[0], i[1]])));
        assert_eq!(g.role(i[1]), Some(&Role::Follower(i[0])));
        assert_eq!(selection(&d, &i), Some(0b011));
    }

    #[test]
    fn analyze_flags_broken_groups() {
        let cases = [
            // Missing part.
            GROUP
                .replace("abc 2/2", "abc 2/3")
                .replace("abc 1/2", "abc 1/3"),
            // Different content.
            GROUP.replacen("SetFontSize 30", "SetFontSize 31", 1),
            // Non-zero-based defence line.
            GROUP.replace("BaseArmour > 0", "BaseArmour > 50"),
            // Bad k/n.
            GROUP.replace("abc 2/2", "abc 1/2"),
            // A comment between the parts.
            GROUP.replace("30\n\n# ff:name", "30\n\n# stray\n\n# ff:name"),
            // Duplicate id elsewhere.
            format!("{GROUP}\n# ff:group abc 1/2\nShow\n"),
        ];
        for t in cases {
            let d = p(&t);
            let g = analyze(&d);
            for id in ids(&d) {
                if d.meta_get(id, GROUP_KEY).is_some() {
                    assert!(g.is_broken(id), "{t}");
                }
            }
        }
    }

    #[test]
    fn selection_grows_and_shrinks_a_group() {
        let src = "Show\n  Rarity Rare\n  SetFontSize 30\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        // Full AR + Full EV: two blocks.
        let m = set_selection(&mut d, &[id], 0b011).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0], id);
        let g = analyze(&d);
        assert_eq!(g.members(id), m);
        assert_eq!(selection(&d, &m), Some(0b011));
        let text = d.serialize();
        assert!(text.contains("# ff:group "), "{text}");
        // Back to one block: marker gone, follower removed.
        let m = set_selection(&mut d, &m, 0b001).unwrap();
        assert_eq!(m, vec![id]);
        assert_eq!(
            d.serialize(),
            "Show\n  Rarity Rare\n  BaseEvasion 0\n  BaseEnergyShield 0\n  SetFontSize 30\n"
        );
        // Empty selection: no defence lines.
        set_selection(&mut d, &m, 0).unwrap();
        assert_eq!(d.serialize(), src);
    }

    #[test]
    fn resync_copies_the_leader_and_keeps_follower_lines() {
        let mut d = p(&GROUP.replace("  BaseArmour 0\n", "  BaseArmour == 0\n"));
        let i = ids(&d);
        d.set_meta(i[0], "name", "Renamed").unwrap();
        resync(&mut d, &i).unwrap();
        let t = d.serialize();
        assert_eq!(t.matches("# ff:name Renamed").count(), 2, "{t}");
        assert!(t.contains("  BaseArmour == 0\n"), "{t}");
        assert_eq!(analyze(&d).members(i[0]).len(), 2);
    }

    #[test]
    fn duplicate_gives_a_new_group_after_the_old_one() {
        let mut d = p(&format!("{GROUP}\nShow\n"));
        let i = ids(&d);
        let copy = duplicate(&mut d, &i[..2], Some("Pure (copy)")).unwrap();
        assert_eq!(copy.len(), 2);
        let all = ids(&d);
        assert_eq!(&all[2..4], &copy[..]);
        let g = analyze(&d);
        assert_eq!(g.members(i[0]), vec![i[0], i[1]]);
        assert_eq!(g.members(copy[0]), copy);
        assert_ne!(gid_of(&d, i[0]), gid_of(&d, copy[0]));
        assert!(d.serialize().starts_with(GROUP));
    }

    #[test]
    fn disabled_groups_are_groups() {
        let mut d = p(GROUP);
        for id in ids(&d) {
            d.set_enabled(id, false).unwrap();
        }
        let g = analyze(&d);
        let i = ids(&d);
        assert_eq!(g.members(i[0]), i);
        assert_eq!(selection(&d, &i), Some(0b011));
        // Re-cover a disabled rule: the new lines come out commented.
        let m = set_selection(&mut d, &i, 0b111).unwrap();
        assert_eq!(m.len(), 3);
        assert!(d.blocks().all(|b| !b.is_enabled()));
        assert_eq!(selection(&d, &m), Some(0b111));
    }

    #[test]
    fn crlf_is_kept_for_new_parts() {
        let src = "Show\r\n  Rarity Rare\r\n";
        let mut d = p(src);
        let id = ids(&d)[0];
        set_selection(&mut d, &[id], 0b011).unwrap();
        let t = d.serialize();
        assert!(!t.replace("\r\n", "").contains('\n'), "{t:?}");
    }
}
