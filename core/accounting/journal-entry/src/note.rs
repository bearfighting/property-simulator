use crate::JournalEntryError;

/// Optional normalized business note attached to a journal entry.
///
/// **Incomplete:** this currently stores only an opaque normalized code. The code
/// vocabulary, whether it describes business purpose or source, and whether more
/// typed fields are needed remain undecided. Consumers must not treat the code as
/// a Slice identity or a persistent source reference.
///
/// Complete this type when its semantics are agreed with representative Slice
/// outputs, including a documented code format, ownership of the code vocabulary,
/// examples for at least two different Slice results, and a clear decision on
/// whether source attribution belongs here or in a separate reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JournalEntryNote {
    code: String,
}

impl JournalEntryNote {
    /// Create a note from a code, trimming surrounding whitespace and converting
    /// ASCII letters to lowercase. The normalized code uses lowercase letters,
    /// digits, and non-empty segments separated by `.`, `-`, or `_`.
    pub fn new(code: impl Into<String>) -> Result<Self, JournalEntryError> {
        let code = code.into().trim().to_ascii_lowercase();
        if !is_valid_code(&code) {
            return Err(JournalEntryError::InvalidNoteCode);
        }

        Ok(Self { code })
    }
}

impl JournalEntryNote {
    pub fn code(&self) -> &str {
        &self.code
    }
}

fn is_valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.split(['.', '-', '_']).all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}
