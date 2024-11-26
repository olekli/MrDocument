use crate::util::make_unique_path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(EnumIter, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    Inbox,
    Outbox,
    Transit,
    Processed,
    Error,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Paths {
    pub path: PathBuf,
    #[serde(rename = "locations")]
    location_mapping: HashMap<Location, String>,
}

impl Default for Paths {
    fn default() -> Paths {
        Paths {
            path: dirs::home_dir()
                .expect("Could not determine home directory")
                .join("MrDocument"),
            location_mapping: HashMap::from(
                Location::iter()
                    .map(|location| (location, location.to_string()))
                    .collect::<HashMap<_, _>>(),
            ),
        }
    }
}

impl Paths {
    pub fn new(path: PathBuf, location_mapping: HashMap<Location, String>) -> Self {
        Paths {
            path,
            location_mapping,
        }
    }

    pub fn with_path(self, path: PathBuf) -> Self {
        Paths {
            path,
            location_mapping: self.location_mapping,
        }
    }

    pub fn make_root(&self, location: Location) -> PathBuf {
        self.path.clone().join(self.get_location_name(location))
    }

    pub async fn make_path_with_filename(&self, location: Location, filename: String) -> PathBuf {
        make_unique_path(self.make_root(location), filename).await
    }

    fn get_location_name(&self, location: Location) -> String {
        self.location_mapping
            .get(&location)
            .cloned()
            .or_else(|| Some(location.to_string()))
            .unwrap()
    }
}
