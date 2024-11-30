use once_cell::sync::OnceCell;
use crate::error::{Error, Result};
use std::env;
use crate::profile::Profile;

static OPENAI_API_KEY: OnceCell<String> = OnceCell::new();

pub fn init() -> Result<()> {
    let config_dir = Profile::get_config_dir()?;
    OPENAI_API_KEY.set(
        env::var("OPENAI_API_KEY")
            .map(|key| key.to_string())
            .or_else(|_: env::VarError| {
                let key = std::fs::read_to_string(config_dir.join("openai-api-key"))?;
                Ok(key.trim_end_matches("\n").to_string())
            })
            .map_err(|_: Error| Error::NoApiKeyError)?
    ).unwrap();

    Ok(())
}

pub fn get() -> &'static String{
    OPENAI_API_KEY.get().unwrap()
}
