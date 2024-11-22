use crate::error::{Error, Result};
use std::path::PathBuf;
use std::ffi::OsString;
use std::fmt;
use tokio::fs;

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Debug)]
pub struct FileObject {
    pub current_location: Location,

    paths: Paths,
    filename: OsString,
}

impl Paths {
    pub fn new(path: PathBuf) -> Self {
        Paths { path }
    }

    pub fn make_root(&self, location: Location) -> PathBuf {
        self.path.clone().join(location.to_string())
    }
}

impl FileObject {
    pub fn new(paths: Paths, filepath: PathBuf) -> Result<Self> {
        Ok(FileObject{
            current_location: Location::Inbox,
            paths,
            filename: filepath.file_name().ok_or_else(|| Error::UnsupportedFileTypeError(filepath.clone()))?.to_os_string(),
        })
    }

    pub fn make_path(&self, location: Location) -> PathBuf {
        self.paths.make_root(location).join(self.filename.clone())
    }

    pub fn make_path_with_new_filename(&self, location: Location, filename: String) -> PathBuf {
        self.paths.make_root(location).join(filename)
    }

    pub fn get_path(&self) -> PathBuf {
        self.make_path(self.current_location)
    }

    pub async fn rename(&mut self, location: Location) -> Result<()> {
        log::debug!("Moving {self:?} to {location:?}");
        let src = self.get_path();
        let dst = self.make_path(location);
        fs::rename(src, dst).await?;
        self.current_location = location;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(PathBuf::from("/home/baz"), PathBuf::from("/home/baz/inbox/foobar.pdf"))]
    fn test(#[case] path: PathBuf, #[case] file: PathBuf) {
        let file = FileObject::new(Paths::new(path), file).unwrap();
        let this_path = file.make_path(Location::Transit);
        assert_eq!(this_path, PathBuf::from("/home/baz/transit/foobar.pdf"));
    }
}
