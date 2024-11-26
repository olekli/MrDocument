use crate::document::DocumentData;
use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

fn make_metdata_entry(key: String, value: String) -> Vec<String> {
    vec![
        "InfoBegin".to_string(),
        format!("InfoKey: {}", key),
        format!("InfoValue: {}", value),
    ]
}

pub async fn update_metadata(
    src: PathBuf,
    dst: PathBuf,
    document_data: &DocumentData,
) -> Result<Vec<()>> {
    log::info!("Updating metadata {src:?}");
    let mut process_in = Command::new("pdftk")
        .arg(src.clone())
        .arg("dump_data_utf8")
        .arg("output")
        .arg("-")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut process_out = Command::new("pdftk")
        .arg(src)
        .arg("update_info_utf8")
        .arg("-")
        .arg("output")
        .arg(dst)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut data_in = process_in.stdout.take().ok_or(Error::RedirectIOError)?;
    let mut data_out = process_out.stdin.take().ok_or(Error::RedirectIOError)?;
    let mut stderr_in = process_in.stderr.take().ok_or(Error::RedirectIOError)?;
    let mut stderr_out = process_out.stderr.take().ok_or(Error::RedirectIOError)?;

    log::debug!("reading data");
    let mut data = String::new();
    data_in.read_to_string(&mut data).await?;

    let updated = data
        + &vec![make_metdata_entry(
            "Keywords".to_string(),
            document_data.keywords.clone().join(", "),
        )]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");

    log::debug!("writing data");
    data_out.write_all(updated.as_bytes()).await?;
    data_out.flush().await?;
    data_out.shutdown().await?;
    drop(data_out);
    let status_out = process_out.wait().await?;

    log::debug!("reading stderr");
    let mut err_in = String::new();
    stderr_in.read_to_string(&mut err_in).await?;
    let mut err_out = String::new();
    stderr_out.read_to_string(&mut err_out).await?;

    log::debug!("awaiting process");
    let status_in = process_in.wait().await?;

    vec![
        status_in
            .success()
            .then_some(())
            .ok_or(Error::MetadataInError(err_in)),
        status_out
            .success()
            .then_some(())
            .ok_or(Error::MetadataOutError(err_out)),
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempdir::TempDir;

    #[rstest]
    #[tokio::test]
    async fn test() {
        let tmp = TempDir::new("mrdocument-test").unwrap();
        let document_data = DocumentData {
            title: "This Title".to_string(),
            summary: "This summary".to_string(),
            class: "This class".to_string(),
            date: "2024-11-11".to_string(),
            keywords: vec!["key1".to_string(), "key2".to_string(), "foo".to_string()],
            content: Some("foobar".to_string()),
        };
        update_metadata(
            PathBuf::from("files/example.pdf"),
            tmp.path().join("example-mod.pdf"),
            &document_data,
        )
        .await
        .unwrap();
    }
}
