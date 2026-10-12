//! Raw line storage: exact text plus the line's own terminator.

/// A line terminator exactly as found in the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
    /// Final line with no terminator.
    None,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::CrLf => "\r\n",
            Eol::None => "",
        }
    }
}

/// One physical line: its text (no terminator, never trimmed) and its terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    text: String,
    eol: Eol,
}

impl Line {
    pub(crate) fn new(text: String, eol: Eol) -> Self {
        Line { text, eol }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn eol(&self) -> Eol {
        self.eol
    }
}

/// Split into (had_bom, lines). Every byte of the input lands in exactly one place.
pub(crate) fn split(input: &str) -> (bool, Vec<Line>) {
    let (bom, rest) = match input.strip_prefix('\u{feff}') {
        Some(r) => (true, r),
        None => (false, input),
    };
    let lines = rest
        .split_inclusive('\n')
        .map(|seg| {
            if let Some(t) = seg.strip_suffix("\r\n") {
                Line::new(t.to_string(), Eol::CrLf)
            } else if let Some(t) = seg.strip_suffix('\n') {
                Line::new(t.to_string(), Eol::Lf)
            } else {
                Line::new(seg.to_string(), Eol::None)
            }
        })
        .collect();
    (bom, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_lines() {
        let (bom, l) = split("a\nb\n");
        assert!(!bom);
        assert_eq!(l.len(), 2);
        assert!(l.iter().all(|x| x.eol() == Eol::Lf));
    }

    #[test]
    fn mixed_and_no_final_newline() {
        let (_, l) = split("a\r\nb\nc");
        let e: Vec<_> = l.iter().map(|x| x.eol()).collect();
        assert_eq!(e, vec![Eol::CrLf, Eol::Lf, Eol::None]);
    }

    #[test]
    fn lone_cr_and_whitespace_kept() {
        let (_, l) = split("a\rb\n");
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].text(), "a\rb");
        let (_, l) = split("\tRarity  Rare   \n");
        assert_eq!(l[0].text(), "\tRarity  Rare   ");
    }

    #[test]
    fn bom_not_in_text() {
        let (bom, l) = split("\u{feff}Show\n");
        assert!(bom);
        assert_eq!(l[0].text(), "Show");
    }

    #[test]
    fn empty_and_only_bom() {
        assert_eq!(split("").1.len(), 0);
        let (bom, l) = split("\u{feff}");
        assert!(bom);
        assert!(l.is_empty());
    }
}
