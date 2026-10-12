//! Lossless PoE2 item-filter document model.
//!
//! Contract: the raw lines (text + their own terminator + a BOM flag) are the source of truth;
//! parsed values are a lookup layer built over them. Serializing an unedited [`Document`] returns
//! the input byte-for-byte. The library never prints and does no file IO; the only `Err` is
//! non-UTF-8 input ([`LoadError`]). Grammar problems become [`FilterDiagnostic`]s.

pub mod diag;
pub mod lines;

pub use diag::{FilterDiagnostic, LoadError, Severity};
pub use lines::{Eol, Line};

const BOM_BYTES: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone)]
pub struct Document {
    bom: bool,
    lines: Vec<Line>,
    diagnostics: Vec<FilterDiagnostic>,
}

/// Parse filter bytes. Fails only if the input is not valid UTF-8.
pub fn parse(input: &[u8]) -> Result<Document, LoadError> {
    let text = std::str::from_utf8(input).map_err(|e| LoadError::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let (bom, lines) = lines::split(text);
    Ok(Document {
        bom,
        lines,
        diagnostics: Vec::new(),
    })
}

impl Document {
    pub fn has_bom(&self) -> bool {
        self.bom
    }
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
    pub fn diagnostics(&self) -> &[FilterDiagnostic] {
        &self.diagnostics
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(BOM_BYTES);
        }
        for l in &self.lines {
            out.extend_from_slice(l.text().as_bytes());
            out.extend_from_slice(l.eol().as_str().as_bytes());
        }
        out
    }
    pub fn serialize(&self) -> String {
        String::from_utf8(self.to_bytes()).expect("lines are valid UTF-8 by construction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(b: &[u8]) {
        assert_eq!(parse(b).unwrap().to_bytes(), b);
    }

    #[test]
    fn round_trips() {
        for b in [
            &b"a\nb\n"[..],
            b"a\r\nb\r\n",
            b"a\r\nb\nc",
            b"\xEF\xBB\xBFShow\n",
            b"",
            b"\xEF\xBB\xBF",
            b"a\rb\n",
            b"\tRarity  Rare   \n",
        ] {
            rt(b);
        }
    }

    #[test]
    fn bom_flag() {
        let d = parse(b"\xEF\xBB\xBF").unwrap();
        assert!(d.has_bom());
        assert!(d.lines().is_empty());
    }

    #[test]
    fn invalid_utf8_is_error() {
        assert!(matches!(
            parse(b"\xFF\xFE"),
            Err(LoadError::InvalidUtf8 { valid_up_to: 0 })
        ));
    }
}
