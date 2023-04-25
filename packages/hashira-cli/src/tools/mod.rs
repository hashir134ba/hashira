// Tools
pub mod node_js;
pub mod tailwindcss;
pub mod wasm_bindgen;

//
use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    path::Path,
    process::{Child, Command},
    str::FromStr,
};

use anyhow::Context;

pub(crate) mod archive;
pub(crate) mod global_cache;
pub(crate) mod utils;

#[derive(Default, Debug, Clone)]
pub struct LoadOptions<'a> {
    pub version: Option<Version>,
    pub install_dir: Option<&'a Path>,
}

/// An external tool.
#[async_trait::async_trait]
pub trait Tool: Sized {
    /// Name of the executable binary of this tool.
    fn binary_name() -> &'static str;

    /// Returns the default version of this tool.
    fn default_version() -> Version;

    /// Returns the arguments used to execute the test command of this tool.
    fn test_version_args() -> &'static [&'static str];

    /// Parses the version of this tool from the given string.
    fn parse_version(s: &str) -> anyhow::Result<Version>;

    /// Additional files to include when loading this tool.
    fn additional_files() -> &'static [&'static str] {
        &[]
    }

    /// Returns the path to the executable.
    fn binary_path(&self) -> &Path;

    /// Loads the tool from cache or install it.
    async fn load() -> anyhow::Result<Self> {
        Self::load_with_options(Default::default()).await
    }

    /// Loads the tool from cache or install it using the given options.
    async fn load_with_options(opts: LoadOptions<'_>) -> anyhow::Result<Self>;
}

pub trait ToolExt: Tool {
    /// Test the version of this tool.
    fn test_version(&self) -> anyhow::Result<Version> {
        let args = Self::test_version_args();
        let output = self.cmd(args).output()?;
        let result = String::from_utf8_lossy(&output.stdout);
        Self::parse_version(&result)
    }

    /// Spawn a command with the given args and returns the child process.
    fn spawn<I, S>(&self, args: I) -> anyhow::Result<Child>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let child = self.cmd(args).spawn()?;
        Ok(child)
    }

    /// Returns a command to execute this tool
    fn cmd<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(self.binary_path());
        cmd.args(args);
        cmd
    }

    /// Returns a asynchronous command to execute this tool
    fn async_cmd<I, S>(&self, args: I) -> tokio::process::Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = tokio::process::Command::new(self.binary_path());
        cmd.args(args);
        cmd
    }
}

/// Test the version of the given binary without any checks if the path actually matches this binary name
pub(crate) fn unchecked_test_version<T: Tool>(
    binary_path: impl AsRef<Path>,
) -> anyhow::Result<Version> {
    let binary_path = binary_path.as_ref();

    anyhow::ensure!(
        binary_path.exists(),
        "binary could not be found: {}",
        binary_path.display()
    );

    let version_args = T::test_version_args();
    let output = Command::new(binary_path).args(version_args).output()?;

    let version_text = String::from_utf8_lossy(&output.stdout);
    let version = T::parse_version(&version_text)?;
    Ok(version)
}

impl<T> ToolExt for T where T: Tool {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    mayor: u32,
    minor: u32,
    patch: Option<u32>,
}

impl Version {
    pub fn new(mayor: u32, minor: u32, patch: Option<u32>) -> Self {
        Version {
            mayor,
            minor,
            patch,
        }
    }

    // FIXME: getters?
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.patch {
            Some(patch) => write!(
                f,
                "{mayor}.{minor}.{patch}",
                mayor = self.mayor,
                minor = self.minor
            ),
            None => write!(f, "{mayor}.{minor}", mayor = self.mayor, minor = self.minor),
        }
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('.').collect::<Vec<_>>();
        anyhow::ensure!(
            parts.len() >= 2 && parts.len() <= 3,
            "invalid string, expected at least 3 digits, but was `{s:?}`"
        );

        let mayor = parts[0]
            .parse()
            .with_context(|| format!("invalid `mayor` in version string: {s:?}"))?;
        let minor = parts[1]
            .parse()
            .with_context(|| format!("invalid `minor` in version string: {s:?}"))?;
        let patch = if parts.len() == 3 {
            let patch = parts[2]
                .parse()
                .with_context(|| format!("invalid `patch` in version string: {s:?}"))?;
            Some(patch)
        } else {
            None
        };

        Ok(Version::new(mayor, minor, patch))
    }
}

/// Represents a collection of command arguments.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct CommandArgs(Vec<OsString>);
impl CommandArgs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.0.push(arg.as_ref().to_os_string());
        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }
}

impl IntoIterator for CommandArgs {
    type Item = OsString;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
