// src/quote/line_kind.rs

#[derive(Debug, PartialEq)]
pub enum LineKind {
    Labour,
    Part,
    Fee,
    Credit,
}

impl LineKind {
    /// Reads the word `label` gives (any case). Used when a saved line is read back.
    pub(crate) fn from_stored(text: &str) -> Option<Self> {
        [
            LineKind::Labour,
            LineKind::Part,
            LineKind::Fee,
            LineKind::Credit,
        ]
        .into_iter()
        .find(|kind| kind.label().eq_ignore_ascii_case(text.trim()))
    }

    pub fn label(&self) -> &str {
        match self {
            LineKind::Labour => "LABOUR",
            LineKind::Part => "PART",
            LineKind::Fee => "FEE",
            LineKind::Credit => "CREDIT",
        }
    }
}
