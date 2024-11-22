use crate::error::{Error, Result};
use std::path::PathBuf;
use std::fmt;

#[derive(Debug)]
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

#[derive(Clone, Debug)]
pub struct Paths {
    path: PathBuf,
}

impl Paths {
    pub fn new(path: PathBuf) -> Self {
        Paths { path }
    }

    pub fn make_path(&self, location: Location, file: PathBuf) -> Result<PathBuf> {
        Ok(self.path.clone().join(location.to_string()).join(
            file.file_name()
                .ok_or_else(|| Error::UnsupportedFileTypeError(self.path.clone()))?,
        ))
    }

    pub fn make_root(&self, location: Location) -> PathBuf {
        self.path.clone().join(location.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(PathBuf::from("/home/baz"), PathBuf::from("/home/baz/inbox/foobar.pdf"))]
    fn test(#[case] path: PathBuf, #[case] file: PathBuf) {
        let paths = Paths::new(path);
        let this_path = paths.make_path(Location::Transit, file).unwrap();
        assert_eq!(this_path, PathBuf::from("/home/baz/transit/foobar.pdf"));
    }
}
