use openai_api_rs::v1::common::GPT4_O;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::paths::Paths;
use crate::error::{Error, Result};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::io::ErrorKind;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ChatGptProfile {
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

fn default_temperature() -> f64 {
    1.0
}

impl Default for ChatGptProfile {
    fn default() -> ChatGptProfile {
        ChatGptProfile {
            model: GPT4_O.to_string(),
            temperature: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Profile {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub chatgpt: ChatGptProfile,
    #[serde(default)]
    pub paths: Paths,
}

fn default_name() -> String {
    "default".to_string()
}

impl Default for Profile {
    fn default() -> Profile {
        Profile {
            name: "default".to_string(),
            chatgpt: ChatGptProfile::default(),
            paths: Paths::default(),
        }
    }
}

impl Profile {
    pub async fn new_from_file(path: PathBuf) -> Result<Profile> {
        Ok(serde_yaml::from_str(&fs::read_to_string(path).await?)?)
    }

    pub async fn write_to_file(&self) -> Result<()> {
        let config_dir = dirs::config_local_dir().ok_or(Error::SkelError)?;
        let path = config_dir.join("MrDocument").join("profile");
        log::debug!("Creating config dir {:?}", path);
        fs::create_dir_all(path.clone()).await?;
        let filepath = path.join(format!("{}.yaml", self.name));
        let file = fs::File::create_new(filepath).await;

        match file {
            Err(err) if err.kind() != ErrorKind::AlreadyExists => Err(Error::from(err)),
            Err(_) => Ok(()),
            Ok(mut file) => Ok(file.write_all(serde_yaml::to_string(self)?.as_bytes()).await?),
        }
    }
}

pub async fn init_default_profile() -> Result<Profile> {
    let profile = Profile::default();
    profile.write_to_file().await?;

    Ok(profile)
}
