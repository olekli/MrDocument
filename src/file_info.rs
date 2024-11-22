use crate::error::{Error, Result};
use base64::{engine::general_purpose, Engine as _};
use std::path::PathBuf;
use std::process::Stdio;
use tempdir::TempDir;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;

#[derive(Debug, PartialEq)]
pub enum FileInfo {
    PDF(PathBuf),
    PNG(PathBuf),
    JPEG(PathBuf),
}

impl FileInfo {
    pub fn new(path: PathBuf) -> Result<Self> {
        let ext = path
            .extension()
            .ok_or_else(|| Error::UnsupportedFileTypeError(path.clone()))?;
        match ext
            .to_ascii_lowercase()
            .to_str()
            .ok_or_else(|| Error::UnsupportedFileTypeError(path.clone()))?
        {
            "pdf" => Ok(FileInfo::PDF(path)),
            //            "png" => Ok(FileInfo::PNG(path)),
            //            "jpg" => Ok(FileInfo::JPEG(path)),
            //            "jpeg" => Ok(FileInfo::JPEG(path)),
            _ => Err(Error::UnsupportedFileTypeError(path.clone())),
        }
    }

    pub async fn base64(&self) -> Result<Vec<String>> {
        match self {
            FileInfo::PDF(path) => FileInfo::process_pdf(path).await,
            FileInfo::PNG(path) => vec![FileInfo::process_image(path).await]
                .into_iter()
                .collect(),
            FileInfo::JPEG(path) => vec![FileInfo::process_image(path).await]
                .into_iter()
                .collect(),
        }
    }

    pub fn mime_type(&self) -> String {
        match self {
            FileInfo::PDF(_) => "image/png".to_string(),
            FileInfo::PNG(_) => "image/png".to_string(),
            FileInfo::JPEG(_) => "image/jpeg".to_string(),
        }
    }

    async fn process_image(image_path: &PathBuf) -> Result<String> {
        let image_data = fs::read(image_path).await?;
        Ok(general_purpose::STANDARD.encode(image_data))
    }

    async fn process_pdf(pdf_path: &PathBuf) -> Result<Vec<String>> {
        let tmp_dir = TempDir::new("mrdocument")?;

        let result = async {
            let pdf_filename = pdf_path
                .file_name()
                .ok_or(Error::Other("Invalid PDF file name".to_string()))?;
            let tmp_pdf_path = tmp_dir.path().join(&pdf_filename);
            fs::copy(pdf_path, &tmp_pdf_path).await?;

            let mut output = Command::new("pdftoppm")
                .arg(&pdf_filename)
                .arg("image")
                .arg("-png")
                .current_dir(&tmp_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let stdout = LinesStream::new(
                BufReader::new(output.stdout.take().ok_or(Error::RedirectIOError)?).lines(),
            );
            let stderr = LinesStream::new(
                BufReader::new(output.stderr.take().ok_or(Error::RedirectIOError)?).lines(),
            );
            let mut out_stream = StreamExt::merge(stdout, stderr);

            let status = output.wait().await?;
            if !status.success() {
                return Err(Error::PdfConversionError(
                    out_stream
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .await?
                        .join("\n"),
                ));
            }

            let mut entries = fs::read_dir(&tmp_dir).await?;
            let mut image_files = Vec::new();

            while let Some(entry) = entries.next_entry().await? {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();

                if file_name.starts_with("image-") && file_name.ends_with(".png") {
                    let page_str = &file_name["image-".len()..file_name.len() - ".png".len()];
                    if let Ok(page_number) = page_str.parse::<u32>() {
                        image_files.push((page_number, entry.path()));
                    }
                }
            }

            image_files.sort_by_key(|(page_number, _)| *page_number);

            let mut results = Vec::new();
            for (_, image_path) in image_files {
                results.push(FileInfo::process_image(&image_path).await?);
            }

            Ok(results)
        }
        .await;

        result
    }
}
