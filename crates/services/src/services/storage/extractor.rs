//! Text extraction from synced cloud files.
//!
//! Dispatches by mime type. Currently supports:
//!   - `text/*`, `application/json`     plain UTF-8 decode
//!   - `application/pdf`                via the `pdf-extract` crate
//!
//! Office formats (docx / xlsx / pptx) and Google-native docs are not yet
//! supported and return `None` so the worker can skip them. The worker uses
//! `is_extractable` upfront to avoid downloading bytes for files we can't
//! parse.

use std::panic::{AssertUnwindSafe, catch_unwind};

/// Mime types we know how to extract text from. Match is case-insensitive on
/// the type half but the subtype is compared lowercase. Anything else returns
/// `false` and is skipped by the worker.
pub fn is_extractable(mime: Option<&str>) -> bool {
    let Some(m) = mime else { return false };
    let m = m.to_ascii_lowercase();
    if m.starts_with("text/") {
        return true;
    }
    matches!(m.as_str(), "application/json" | "application/pdf")
}

/// Best-effort text extraction. Returns `None` (rather than `Err`) for mime
/// types we don't handle so the worker can simply skip the file. Returns an
/// error only when an extractor recognizes the format but fails partway
/// through — the worker treats that as a hard failure for that one file.
pub fn extract_text(mime: Option<&str>, bytes: &[u8]) -> Result<Option<String>, ExtractError> {
    let Some(m) = mime.map(|s| s.to_ascii_lowercase()) else {
        return Ok(None);
    };

    if m.starts_with("text/") || m == "application/json" {
        return Ok(Some(decode_utf8_lossy(bytes)));
    }
    if m == "application/pdf" {
        return extract_pdf(bytes).map(Some);
    }
    Ok(None)
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("PDF extraction failed: {0}")]
    Pdf(String),
}

/// Decode bytes as UTF-8, lossily replacing invalid sequences. Truncates any
/// trailing nulls/whitespace.
fn decode_utf8_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// Wrap `pdf-extract` in `catch_unwind` because the crate panics on some
/// malformed PDFs rather than returning Err. We'd rather skip than crash the
/// background worker.
fn extract_pdf(bytes: &[u8]) -> Result<String, ExtractError> {
    let owned = bytes.to_vec();
    let result =
        catch_unwind(AssertUnwindSafe(|| pdf_extract::extract_text_from_mem(&owned)));
    match result {
        Ok(Ok(text)) => Ok(text.trim().to_string()),
        Ok(Err(e)) => Err(ExtractError::Pdf(e.to_string())),
        Err(_) => Err(ExtractError::Pdf("panic during extraction".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_plain_extracts_utf8() {
        let out = extract_text(Some("text/plain"), b"hello world").unwrap();
        assert_eq!(out.as_deref(), Some("hello world"));
    }

    #[test]
    fn unknown_mime_returns_none() {
        let out = extract_text(Some("image/png"), &[0u8; 8]).unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn is_extractable_text_pdf_json_true() {
        assert!(is_extractable(Some("text/markdown")));
        assert!(is_extractable(Some("application/pdf")));
        assert!(is_extractable(Some("application/json")));
        assert!(!is_extractable(Some("image/jpeg")));
        assert!(!is_extractable(None));
    }
}
