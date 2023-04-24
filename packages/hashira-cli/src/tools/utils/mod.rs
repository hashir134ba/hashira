use anyhow::Context;
use cap_directories::{ambient_authority, ProjectDirs};
use cap_std::fs::Dir;
use futures::StreamExt;
use reqwest::Client;
use std::process::Command;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

/// Returns the cache directory.
pub fn cache_dir() -> anyhow::Result<Dir> {
    let authority = ambient_authority();
    let dir = ProjectDirs::from("dev", "hashira-rs", "hashira", authority)
        .context("failed finding project directory")?
        .cache_dir()?;

    Ok(dir)
}

pub(crate) fn cmd<I, S>(bin_path: impl AsRef<Path>, args: I) -> anyhow::Result<Command>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let path = dunce::canonicalize(bin_path.as_ref())?;

    anyhow::ensure!(
        path.exists(),
        "executable was not found: {}",
        path.display()
    );

    let mut command = Command::new(path);
    command.args(args);
    Ok(command)
}

/// Executes the given command and returns the process.
#[allow(dead_code)]
pub fn exec<I, S>(bin_path: impl AsRef<Path>, args: I) -> anyhow::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let process = cmd(bin_path, args)?.spawn()?;
    Ok(process)
}

/// Executes the given command and returns the result of stdout.
pub fn exec_and_get_output<I, S>(bin_path: impl AsRef<Path>, args: I) -> anyhow::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = cmd(bin_path, args)?.output()?;
    let result = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(result)
}

/// Download a file and write the content to the destination.
pub async fn download<W>(url: &str, dest: &mut W) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let client = Client::new();
    let res = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download: {url}"))?;

    let mut stream = res.bytes_stream();
    let mut writer = BufWriter::new(dest);

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.context("failed to download file")?;
        writer
            .write_all(&bytes)
            .await
            .context("failed to write file")?;

        writer.flush().await?;
    }

    Ok(())
}

/// Downloads a file to the give path.
pub async fn download_to_file(url: &str, file_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let file_path = file_path.as_ref();

    if let Some(parent) = file_path.parent() {
        anyhow::ensure!(
            parent.exists(),
            "parent directory does not exists: {}",
            parent.display()
        );
    }

    let mut file = tokio::fs::File::create(file_path).await?;
    download(url, &mut file).await?;
    Ok(file_path.to_path_buf())
}

/// Downloads a file to the given directory.
pub async fn download_to_dir(url: &str, target_dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    fn get_file_name(url: &str) -> Option<String> {
        url.split('/').last().map(|s| s.to_owned())
    }

    let dir = target_dir.as_ref();
    anyhow::ensure!(dir.is_dir(), "`{}` is not a directory", dir.display());

    let file_name = get_file_name(url)
        .ok_or_else(|| anyhow::anyhow!("unable to get file name from the url: {url}"))?;
    let file_path = dir.join(file_name);
    download_to_file(url, file_path).await
}

/// Downloads and extract the given file.
pub async fn download_and_extract(
    url: &str,
    file_name: &str,
    dest: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let dest_dir = dest.as_ref();

    anyhow::ensure!(
        dest_dir.is_dir(),
        "`{}` is not a directory",
        dest_dir.display()
    );

    // Create the directory
    tokio::fs::create_dir_all(dest_dir).await?;

    // Download and extract
    let downloaded = download_to_dir(url, &dest_dir).await?;
    let temp_path = tempfile::TempPath::from_path(downloaded); // download to a temporary file

    let Some(decompressor) = crate::tools::decompress::Decompressor::get(&temp_path)? else {
        anyhow::bail!("unable to find decompressor for: {}", temp_path.display());
    };

    let decompressed = decompressor.extract_file(file_name, dest_dir)?;
    Ok(decompressed)
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::tools::utils::cache_dir;

    #[tokio::test]
    async fn test_download() {
        let named_temp = create_temp_file().await;
        let temp_file = named_temp.path().to_path_buf();
        let mut file = tokio::fs::File::create(&temp_file).await.unwrap();
        super::download(
            "https://raw.githubusercontent.com/Neo-Ciber94/hashira/main/README.md",
            &mut file,
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(&temp_file).await.unwrap();
        assert!(
            content.starts_with("# hashira"),
            "actual contents: `{}`",
            content
        );
    }

    #[tokio::test]
    async fn test_download_to_file() {
        let file_path = Path::new("temp/test/readme_test.md");
        let dest_path = super::download_to_file(
            "https://raw.githubusercontent.com/Neo-Ciber94/hashira/main/README.md",
            file_path,
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(&dest_path).await.unwrap();
        assert!(
            content.starts_with("# hashira"),
            "actual contents: `{}`",
            content
        );
    }

    #[tokio::test]
    async fn test_download_to_dir() {
        let dest = Path::new("temp/test");
        let dest_path = super::download_to_dir(
            "https://raw.githubusercontent.com/Neo-Ciber94/hashira/main/README.md",
            dest,
        )
        .await
        .unwrap();

        assert!(dest_path.ends_with("README.md"));

        let content = tokio::fs::read_to_string(&dest_path).await.unwrap();
        assert!(
            content.starts_with("# hashira"),
            "actual contents: `{}`",
            content
        );
    }

    #[tokio::test]
    async fn test_download_and_decompress_tar_gz() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        let downloaded = super::download_and_extract(
            "https://github.com/Neo-Ciber94/sample_files/raw/main/file.tar.gz",
            "file.txt",
            dir_path,
        )
        .await
        .unwrap();

        assert!(downloaded.ends_with("file.txt"));

        let contents = tokio::fs::read_to_string(&downloaded).await.unwrap();
        assert_eq!(contents, "Hello World!\n", "actual contents: `{contents}`");
    }

    #[tokio::test]
    async fn test_download_and_decompress_zip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        let downloaded = super::download_and_extract(
            "https://github.com/Neo-Ciber94/sample_files/raw/main/file.zip",
            "file.txt",
            dir_path,
        )
        .await
        .unwrap();

        assert!(downloaded.ends_with("file.txt"));

        let contents = tokio::fs::read_to_string(&downloaded).await.unwrap();
        assert_eq!(contents, "Hello World!\n", "actual contents: `{contents}`");
    }

    #[tokio::test]
    async fn test_download_to_cache_dir() {
        let cache_dir = cache_dir().unwrap();
        let dir = cache_dir.canonicalize(".").unwrap();
        let temp_dir = tempfile::tempdir_in(dir).unwrap();

        let downloaded = super::download_to_dir(
            "https://github.com/Neo-Ciber94/sample_files/raw/main/file.txt",
            temp_dir.path(),
        )
        .await
        .unwrap();

        let contents = std::fs::read_to_string(&downloaded).unwrap();
        assert_eq!(contents, "Hello World!\n", "actual contents: `{contents}`")
    }

    #[tokio::test]
    async fn test_download_and_extract_to_cache_dir() {
        let cache_dir = cache_dir().unwrap();
        let dir = cache_dir.canonicalize(".").unwrap();
        let temp_dir = tempfile::tempdir_in(dir).unwrap();

        let downloaded = super::download_and_extract(
            "https://github.com/Neo-Ciber94/sample_files/raw/main/file.tar.gz",
            "file.txt",
            temp_dir.path(),
        )
        .await
        .unwrap();

        let contents = std::fs::read_to_string(&downloaded).unwrap();
        assert_eq!(contents, "Hello World!\n", "actual contents: `{contents}`")
    }

    async fn create_temp_file() -> tempfile::NamedTempFile {
        let path = Path::new("temp/test");

        if !path.exists() {
            tokio::fs::create_dir_all(path)
                .await
                .expect("failed to create test dir");
        }

        let temp_file = tempfile::NamedTempFile::new_in(path).unwrap();
        temp_file
    }
}