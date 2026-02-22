use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokemonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error in {file}: {source}")]
    JsonParse {
        file: String,
        source: serde_json::Error,
    },

    #[error("Provider '{0}' not found")]
    ProviderNotFound(String),

    #[error("Failed to fetch pricing data: {0}")]
    PricingFetch(String),
}

pub type Result<T> = std::result::Result<T, TokemonError>;
