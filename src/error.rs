use std::fmt;

#[derive(Debug, Clone)]
pub struct ProtocolError(pub(crate) String);

impl std::error::Error for ProtocolError {}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone)]
pub enum LspServerError<E> {
    ProtocolError(ProtocolError),
    ServerError(E),
}

// Note: can't implement std::error::Error nicely without specialization

impl<E> From<ProtocolError> for LspServerError<E> {
    fn from(error: ProtocolError) -> Self {
        LspServerError::ProtocolError(error)
    }
}
