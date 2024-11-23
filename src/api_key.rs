use once_cell::sync::OnceCell;
use crate::error::{Error, Result};
use std::path::PathBuf;
use std::env;

static OPENAI_API_KEY: OnceCell<String> = OnceCell::new();

pub fn init(path: PathBuf) -> Result<()> {
    OPENAI_API_KEY.set(
        env::var("OPENAI_API_KEY")
            .map(|key| key.to_string())
            .or_else(|_: env::VarError| {
                let key = std::fs::read_to_string(path.join(".openai-api-key"))?;
                Ok(key.trim_end_matches("\n").to_string())
            })
            .map_err(|_: Error| Error::NoApiKeyError)?
    ).unwrap();

    Ok(())
}

pub fn get() -> &'static String{
    OPENAI_API_KEY.get().unwrap()
}
