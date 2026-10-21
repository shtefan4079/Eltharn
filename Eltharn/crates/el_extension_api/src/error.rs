use core::fmt;
pub const PREVIEW_CHARS: usize = 48;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    Empty,
    TooLong,
    BadLength,
    MissingSeparator,
    ExtraSeparator,
    EmptyPart,
    BadStart,
    BadChar,
    BadEnd,
    BadSegment,
    ForbiddenValue,
    BadPrefix,
    BadVersion,
    BadNumber,
    NumberOutOfRange,
    LeadingZero,
    TooDeep,
    Zero,
}
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ErrorKind::Empty => "empty input",
            ErrorKind::TooLong => "input exceeds maximum length",
            ErrorKind::BadLength => "input has wrong exact length",
            ErrorKind::MissingSeparator => "required separator missing",
            ErrorKind::ExtraSeparator => "too many separators",
            ErrorKind::EmptyPart => "empty part or segment",
            ErrorKind::BadStart => "invalid first character",
            ErrorKind::BadChar => "invalid character",
            ErrorKind::BadEnd => "invalid trailing character",
            ErrorKind::BadSegment => "invalid segment",
            ErrorKind::ForbiddenValue => "forbidden value",
            ErrorKind::BadPrefix => "invalid prefix",
            ErrorKind::BadVersion => "invalid version",
            ErrorKind::BadNumber => "invalid number",
            ErrorKind::NumberOutOfRange => "number out of range",
            ErrorKind::LeadingZero => "leading zero not allowed",
            ErrorKind::TooDeep => "path exceeds maximum depth",
            ErrorKind::Zero => "value must be positive",
        };
        f.write_str(s)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    ty: &'static str,
    kind: ErrorKind,
    input_len: usize,
    position: Option<usize>,
    preview: String,
}
impl Error {
    pub(crate) fn new(ty: &'static str, kind: ErrorKind, input: &str) -> Self {
        Self::at_opt(ty, kind, input, None)
    }
    pub(crate) fn at(ty: &'static str, kind: ErrorKind, input: &str, position: usize) -> Self {
        Self::at_opt(ty, kind, input, Some(position))
    }
    fn at_opt(ty: &'static str, kind: ErrorKind, input: &str, position: Option<usize>) -> Self {
        Self {
            ty,
            kind,
            input_len: input.len(),
            position,
            preview: preview_of(input),
        }
    }
    pub fn type_name(&self) -> &'static str {
        self.ty
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn input_len(&self) -> usize {
        self.input_len
    }
    pub fn position(&self) -> Option<usize> {
        self.position
    }
    pub fn preview(&self) -> &str {
        &self.preview
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}: {}", self.ty, self.kind)?;
        write!(f, " (input {} bytes", self.input_len)?;
        if let Some(pos) = self.position {
            write!(f, ", byte {pos}")?;
        }
        write!(f, ", input {:?})", self.preview)
    }
}
impl std::error::Error for Error {}
fn preview_of(input: &str) -> String {
    let mut chars = input.chars();
    let mut out = String::new();
    for _ in 0..PREVIEW_CHARS {
        match chars.next() {
            Some(ch) => out.push(ch),
            None => return out,
        }
    }
    if chars.next().is_some() {
        out.pop();
        out.push('…');
    }
    out
}
