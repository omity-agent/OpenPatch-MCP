use alloc::borrow::Cow;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sequence {
    Cr,
    CrLf,
    Lf,
}
impl Sequence {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cr => "\r",
            Self::CrLf => "\r\n",
            Self::Lf => "\n",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineEndings {
    selected: Sequence,
    mixed: bool,
}
impl LineEndings {
    pub(crate) fn detect(contents: &str) -> Self {
        let bytes = contents.as_bytes();
        let mut detected = None;
        let mut mixed = false;
        let mut index = 0;
        while index < bytes.len() {
            let Some(&byte) = bytes.get(index) else {
                panic!("line ending scan index must be in bounds");
            };
            let (sequence, width) = match byte {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => (Some(Sequence::CrLf), 2),
                b'\r' => (Some(Sequence::Cr), 1),
                b'\n' => (Some(Sequence::Lf), 1),
                _ => (None, 1),
            };
            if let Some(current) = sequence {
                match detected {
                    None => detected = Some(current),
                    Some(previous) if previous != current => mixed = true,
                    Some(_) => {}
                }
            }
            index += width;
        }
        Self {
            selected: if mixed {
                Sequence::Lf
            } else {
                detected.unwrap_or(Sequence::Lf)
            },
            mixed,
        }
    }
    pub(crate) fn normalize(contents: &str) -> Cow<'_, str> {
        if !contents.as_bytes().contains(&b'\r') {
            return Cow::Borrowed(contents);
        }
        let bytes = contents.as_bytes();
        let mut normalized = String::with_capacity(contents.len());
        let mut copied_until = 0;
        let mut index = 0;
        while index < bytes.len() {
            let Some(&byte) = bytes.get(index) else {
                panic!("line ending normalization index must be in bounds");
            };
            if byte != b'\r' {
                index += 1;
                continue;
            }
            let Some(span) = contents.get(copied_until..index) else {
                panic!("carriage return offsets must be UTF-8 boundaries");
            };
            normalized.push_str(span);
            normalized.push('\n');
            index += if bytes.get(index + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            copied_until = index;
        }
        let Some(remainder) = contents.get(copied_until..) else {
            panic!("line ending offset must be a UTF-8 boundary");
        };
        normalized.push_str(remainder);
        Cow::Owned(normalized)
    }
    pub(crate) fn normalize_owned(contents: String) -> String {
        if !contents.as_bytes().contains(&b'\r') {
            return contents;
        }
        Self::normalize(&contents).into_owned()
    }
    pub(crate) fn render(self, normalized: String) -> String {
        match self.selected {
            Sequence::Cr => normalized.replace('\n', Sequence::Cr.as_str()),
            Sequence::CrLf => normalized.replace('\n', Sequence::CrLf.as_str()),
            Sequence::Lf => normalized,
        }
    }
    pub(crate) const fn is_mixed(self) -> bool {
        self.mixed
    }
}
pub(crate) fn line_count(contents: &str) -> usize {
    if contents.is_empty() {
        0
    } else {
        bytecount::count(contents.as_bytes(), b'\n') + usize::from(!contents.ends_with('\n'))
    }
}
pub(crate) fn character_count(contents: &str) -> usize {
    if contents.is_ascii() {
        contents.len()
    } else {
        bytecount::num_chars(contents.as_bytes())
    }
}
#[cfg(test)]
#[path = "../test/text.rs"]
mod tests;
