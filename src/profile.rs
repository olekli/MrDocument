use crate::error::{Error, Result};
use crate::paths::Paths;
use openai_api_rs::v1::common::GPT4_O;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChatGptProfile {
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: u8,
    pub additional_instructions: Vec<String>,
}

fn default_temperature() -> u8 {
    100
}

impl Default for ChatGptProfile {
    fn default() -> ChatGptProfile {
        ChatGptProfile {
            model: GPT4_O.to_string(),
            temperature: 100,
            additional_instructions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub chatgpt: ChatGptProfile,
    pub paths: Paths,
    #[serde(default)]
    pub polling: bool,
}

impl Default for Profile {
    fn default() -> Profile {
        Profile {
            name: "default".to_string(),
            chatgpt: ChatGptProfile::default(),
            paths: Paths::default(),
            polling: false,
        }
    }
}

impl Profile {
    pub async fn new_from_file(path: PathBuf) -> Result<Profile> {
        Ok(serde_yaml::from_str(&fs::read_to_string(path).await?)?)
    }

    pub fn with_path(self, path: PathBuf) -> Profile {
        Profile {
            paths: self.paths.with_path(path),
            ..self
        }
    }

    pub async fn write_to_file(&self) -> Result<()> {
        let path = Profile::get_profile_dir()?;
        log::debug!("Creating config dir {:?}", path);
        fs::create_dir_all(path.clone()).await?;
        let filepath = path.join(format!("{}.yaml", self.name));
        let file = fs::File::create_new(filepath).await;

        match file {
            Err(err) if err.kind() != ErrorKind::AlreadyExists => Err(Error::from(err)),
            Err(_) => Ok(()),
            Ok(mut file) => Ok(file
                .write_all(serde_yaml::to_string(self)?.as_bytes())
                .await?),
        }
    }

    pub fn get_profile_dir() -> Result<PathBuf> {
        let path = Profile::get_config_dir()?.join("profile");

        Ok(path)
    }

    pub fn get_config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_local_dir().ok_or(Error::SkelError)?;
        let path = config_dir.join("MrDocument");

        Ok(path)
    }

    pub async fn init_default_profile() -> Result<()> {
        let profile = Profile::default();
        profile.write_to_file().await?;

        Ok(())
    }
}
