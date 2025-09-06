use thiserror::Error;

/// Gateway-wide error model for uniform HTTP/JSON mapping.
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("{0}")]
    Message(String),
}

impl From<anyhow::Error> for GatewayError {
    fn from(e: anyhow::Error) -> Self {
        GatewayError::Message(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_displays_message() {
        let e = GatewayError::Message("boom".into());
        assert_eq!(e.to_string(), "boom");
    }
}


