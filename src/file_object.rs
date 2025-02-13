use crate::error::{Error, Result};
use crate::paths::{Location, Paths};
use crate::util::file_exists;
use crate::util::{make_unique_path, move_file};
use std::ffi::OsString;
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[derive(Clone, Debug)]
pub struct FileObject {
    pub current_location: Location,

    paths: Paths,
    filename: OsString,
}

impl FileObject {
    pub fn new(paths: Paths, filepath: PathBuf) -> Result<Self> {
        let file = FileObject {
            current_location: Location::Inbox,
            paths,
            filename: filepath
                .file_name()
                .ok_or_else(|| Error::UnsupportedFileTypeError(filepath.clone()))?
                .to_os_string(),
        };
        for location in Location::iter() {
            if location != Location::Inbox {
                if file_exists(&file.make_path(location)) {
                    return Err(Error::FileExists(filepath));
                }
            }
        }
        Ok(file)
    }

    pub fn make_path(&self, location: Location) -> PathBuf {
        self.paths.make_root(location).join(self.filename.clone())
    }

    pub async fn make_path_with_new_filename(
        &self,
        location: Location,
        path: PathBuf,
        filename: String,
    ) -> Result<PathBuf> {
        Ok(make_unique_path(self.paths.make_root(location).join(path), filename).await?)
    }

    pub fn get_path(&self) -> PathBuf {
        self.make_path(self.current_location)
    }

    pub async fn rename(&mut self, location: Location) -> Result<()> {
        log::debug!("Moving {self:?} to {location:?}");
        let src = self.get_path();
        let dst = self.make_path(location);
        move_file(&src, &dst).await?;
        self.current_location = location;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(
        PathBuf::from("/home/baz"),
        PathBuf::from("/home/baz/inbox/foobar.pdf")
    )]
    fn test(#[case] path: PathBuf, #[case] file: PathBuf) {
        let file = FileObject::new(Paths::default().with_path(path), file).unwrap();
        let this_path = file.make_path(Location::Transit);
        assert_eq!(this_path, PathBuf::from("/home/baz/transit/foobar.pdf"));
    }
}
