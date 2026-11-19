// src/repair_order/notes.rs: the rules for the free-text paragraphs on a repair order
//
// The complaint, the cause and the correction are optional paragraphs written by people. One
// function cleans all three, so they can never follow different rules, and the database layer
// uses the same function to check what it reads back.

use crate::errors::{QuoteError, QuoteResult};

/// The most characters one paragraph can hold (about two pages of text).
pub const MAX_NOTE_CHARS: usize = 4000;

/// Cleans one paragraph. `None` or only spaces means "nothing written": `Ok(None)`.
///
/// - Spaces and blank lines at the start and end are dropped.
/// - Windows line endings (`\r\n`) become `\n`, so the same words are always stored the same way.
/// - New lines and tabs are fine; every other control character is refused (they would break a
///   printed page and mean nothing to a reader).
/// - At most `MAX_NOTE_CHARS` characters (counted as characters, not bytes, so accents are fair).
///
/// `field` is Quoto's own word for the paragraph; it is the only text in the error messages.
pub(super) fn clean_note(field: &'static str, text: Option<&str>) -> QuoteResult<Option<String>> {
    let Some(text) = text else {
        return Ok(None);
    };
    let text = text.replace("\r\n", "\n");
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(QuoteError::RepairOrderTextInvalid { field });
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(QuoteError::RepairOrderTextTooLong {
            field,
            max: MAX_NOTE_CHARS,
        });
    }
    Ok(Some(text.to_string()))
}

/// What the database reads back must already be clean: anything else was written behind
/// Quoto's back. (`None` stays `None`; a stored blank or untidy paragraph is "damaged".)
pub(super) fn is_stored_clean(field: &'static str, text: &Option<String>) -> bool {
    match text {
        None => true,
        Some(text) => matches!(
            clean_note(field, Some(text)),
            Ok(Some(clean)) if clean == *text
        ),
    }
}
