use clap::Args;
use std::path::PathBuf;

use super::{wasm_opt_level::WasmOptimizationLevel, DevOptions, RunOptions};

// directories and files included as default in the `public_dir` if not valid is specified.
pub const DEFAULT_INCLUDES: &[&str] = &["public/", "styles/", "favicon.ico"];

#[derive(Args, Debug, Clone)]
pub struct BuildOptions {
    #[arg(short, long, help = "Base directory for the artifacts")]
    pub target_dir: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Directory relative to the `target_dir` where the static files will be serve from",
        default_value = "public"
    )]
    pub public_dir: PathBuf,

    #[arg(
        long,
        help = "Build artifacts in release mode, with optimizations",
        default_value_t = false
    )]
    pub release: bool,

    #[arg(
        long,
        help = "A list of files and directories to copy in the `public_dir`, by default include the `public/`, `styles/` and `favicon.ico` if found"
    )]
    pub include: Vec<PathBuf>,

    #[arg(
        long,
        help = "Allow to include files outside the current directory",
        default_value_t = false
    )]
    pub allow_include_external: bool,

    #[arg(
        long,
        help = "Allow to include files inside src/ directory",
        default_value_t = false
    )]
    pub allow_include_src: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Whether if output the commands output"
    )]
    pub quiet: bool,

    // TODO: Allow not value and default to `s`
    #[arg(
        long,
        help = "Optimization level for the wasm, possible values: 0, 1, 2, 3, 4, s, z"
    )]
    pub opt_level: Option<WasmOptimizationLevel>,

    #[arg(
        long,
        default_value = "global.css",
        help = "Path to the css entry file, this file can be scss or sass"
    )]
    pub styles: PathBuf,
}

impl BuildOptions {
    pub fn resolved_target_dir(&self) -> anyhow::Result<PathBuf> {
        match &self.target_dir {
            Some(path) => Ok(path.clone()),
            None => crate::utils::get_target_dir(),
        }
    }

    pub fn profile_target_dir(&self) -> anyhow::Result<PathBuf> {
        let mut dir = self.resolved_target_dir()?;

        if self.release {
            dir.push("release");
        } else {
            dir.push("debug");
        };

        Ok(dir)
    }
}

impl From<&DevOptions> for BuildOptions {
    fn from(dev_opts: &DevOptions) -> Self {
        Self {
            target_dir: dev_opts.target_dir.clone(),
            public_dir: dev_opts.public_dir.clone(),
            release: dev_opts.release,
            include: dev_opts.include.clone(),
            allow_include_external: dev_opts.allow_include_external,
            allow_include_src: dev_opts.allow_include_src,
            quiet: dev_opts.quiet,
            opt_level: dev_opts.opt_level,
            styles: dev_opts.styles.clone(),
        }
    }
}

impl From<&RunOptions> for BuildOptions {
    fn from(run_opts: &RunOptions) -> Self {
        Self {
            target_dir: run_opts.target_dir.clone(),
            public_dir: run_opts.public_dir.clone(),
            release: run_opts.release,
            include: run_opts.include.clone(),
            allow_include_external: run_opts.allow_include_external,
            allow_include_src: run_opts.allow_include_src,
            quiet: run_opts.quiet,
            opt_level: run_opts.opt_level,
            styles: run_opts.styles.clone(),
        }
    }
}
