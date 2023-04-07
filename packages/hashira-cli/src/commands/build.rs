use crate::pipelines::copy_files::CopyFilesPipeline;
use crate::pipelines::Pipeline;
use crate::utils::{get_cargo_lib_name, get_target_dir};
use anyhow::Context;
use clap::Args;
use glob::glob;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;

// directories and files included as default in the `public_dir` if not valid is specified.
const DEFAULT_INCLUDES: &[&str] = &["public/*", "favicon.ico"];

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
        help = "A list of files to copy in the `public_dir`, by default include the `public/` and `favicon.ico` if found"
    )]
    pub include: Vec<String>,

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
}

impl BuildOptions {
    pub fn resolved_target_dir(&self) -> anyhow::Result<PathBuf> {
        match &self.target_dir {
            Some(path) => Ok(path.clone()),
            None => get_target_dir(),
        }
    }
}

pub async fn build(opts: BuildOptions) -> anyhow::Result<()> {
    log::info!("Build started");

    build_server(&opts).await?;
    build_wasm(&opts).await?;
    Ok(())
}

pub async fn build_server(opts: &BuildOptions) -> anyhow::Result<()> {
    log::info!("Building server...");
    cargo_build(opts).await?;

    log::info!("✅ Build server done!");
    Ok(())
}

pub async fn build_wasm(opts: &BuildOptions) -> anyhow::Result<()> {
    log::info!("Building wasm...");
    prepare_public_dir(&opts).await?;

    log::info!("Running cargo build --target wasm32-unknown-unknown...");
    cargo_build_wasm(&opts).await?;

    log::info!("Generating wasm bindings...");
    wasm_bindgen_build(&opts).await?;

    log::info!("Copying files to public directory...");
    include_files(&opts).await?;

    log::info!("✅ Build wasm done!");

    Ok(())
}

async fn prepare_public_dir(opts: &BuildOptions) -> anyhow::Result<()> {
    let mut public_dir = match &opts.target_dir {
        Some(path) => path.clone(),
        None => get_target_dir()?,
    };

    if opts.release {
        public_dir.push("release");
    } else {
        public_dir.push("debug");
    }

    public_dir.push(&opts.public_dir);

    log::debug!("Trying to clean public directory: {}", public_dir.display());

    if public_dir.exists() {
        log::info!("Preparing public directory: {}", public_dir.display());
        tokio::fs::remove_dir_all(&public_dir)
            .await
            .with_context(|| format!("failed to remove dir: {}", public_dir.display()))?;
    } else {
        log::debug!("Public directory was not found");
    }

    Ok(())
}

async fn cargo_build(opts: &BuildOptions) -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");

    // args
    cmd.arg("build");

    if opts.quiet {
        cmd.arg("--quiet");
    }

    // target dir
    let target_dir = opts.resolved_target_dir()?;
    log::debug!("target dir: {}", target_dir.display());

    cmd.arg("--target-dir");
    cmd.arg(target_dir);

    // release mode?
    if opts.release {
        cmd.arg("--release");
    }

    // Run
    let mut child = cmd.spawn()?;
    let status = child.wait().await?;
    anyhow::ensure!(status.success(), "failed to build crate");

    Ok(())
}

async fn cargo_build_wasm(opts: &BuildOptions) -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");

    // args
    cmd.arg("build")
        .args(["--target", "wasm32-unknown-unknown"]);

    if opts.quiet {
        cmd.arg("--quiet");
    }

    // target dir
    let target_dir = opts.resolved_target_dir()?;
    log::debug!("target dir: {}", target_dir.display());

    cmd.arg("--target-dir");
    cmd.arg(target_dir);

    // release mode?
    if opts.release {
        cmd.arg("--release");
    }

    // Run
    let mut child = cmd.spawn()?;
    let status = child.wait().await?;
    anyhow::ensure!(status.success(), "failed to build wasm crate");

    Ok(())
}

async fn wasm_bindgen_build(opts: &BuildOptions) -> anyhow::Result<()> {
    // TODO: Download wasm-bindgen if doesn't exists on the machine
    let mut cmd = Command::new("wasm-bindgen");

    // args
    cmd.args(["--target", "web"]).arg("--no-typescript");

    // out dir
    let mut out_dir = opts.resolved_target_dir()?.join({
        if opts.release {
            "release"
        } else {
            "debug"
        }
    });

    out_dir.push(&opts.public_dir);
    log::debug!("wasm-bindgen out-dir: {}", out_dir.display());

    cmd.arg("--out-dir").arg(out_dir);

    // wasm to bundle
    // The wasm is located in ${target_dir}/wasm32-unknown-unknown/{profile}/{project_name}.wasm
    let wasm_target_dir = opts.resolved_target_dir()?.join({
        if opts.release {
            "wasm32-unknown-unknown/release"
        } else {
            "wasm32-unknown-unknown/debug"
        }
    });

    let mut wasm_dir = wasm_target_dir.clone();
    let lib_name = get_cargo_lib_name().context("Failed to get lib name")?;
    wasm_dir.push(format!("{lib_name}.wasm"));
    log::debug!("wasm file dir: {}", wasm_dir.display());

    cmd.arg(wasm_dir);

    // Run
    let mut child = cmd.spawn()?;
    let status = child.wait().await?;
    anyhow::ensure!(status.success(), "failed to build wasm");

    Ok(())
}

struct IncludeFiles {
    glob: String,
    is_default: bool,
}

async fn include_files(opts: &BuildOptions) -> anyhow::Result<()> {
    let include_files: Vec<IncludeFiles>;

    if opts.include.is_empty() {
        include_files = DEFAULT_INCLUDES
            .into_iter()
            .map(|s| (*s).to_owned())
            .map(|glob| IncludeFiles {
                glob,
                is_default: true,
            })
            .collect::<Vec<_>>();
    } else {
        include_files = opts
            .include
            .clone()
            .into_iter()
            .map(|glob| IncludeFiles {
                glob,
                is_default: false,
            })
            .collect::<Vec<_>>();
    }

    let mut dest_dir = opts.resolved_target_dir()?.join({
        if opts.release {
            "release"
        } else {
            "debug"
        }
    });

    dest_dir.push(&opts.public_dir);

    process_files(&include_files, dest_dir.as_path(), opts)
        .await
        .context("Failed to copy files")?;

    Ok(())
}

async fn process_files(
    files: &[IncludeFiles],
    dest_dir: &Path,
    opts: &BuildOptions,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().context("failed to get current working directory")?;
    let mut globs = Vec::new();

    for file in files {
        if !file.is_default {
            let path = Path::new(&file.glob);
            if path.is_dir() {
                anyhow::ensure!(path.exists(), "directory {} does not exist", path.display());
            }
        }

        globs.push(&file.glob);
    }

    let mut files = Vec::new();

    for glob_str in globs {
        for entry in glob(glob_str).expect("Failed to read glob pattern") {
            let path = entry?;

            // We ignore directories
            if path.is_dir() {
                continue;
            }

            if !opts.allow_include_external && is_outside_directory(&cwd, &path)? {
                log::error!("{} is outside {}", path.display(), cwd.display());

                anyhow::bail!(
                    "Path to include cannot be outside the current directory, use `--allow-include-external` to include files outside the current directory"
                );
            }

            if !opts.allow_include_src && is_inside_src(&cwd, &path)? {
                log::error!("{} is inside `src` directory", path.display());

                anyhow::bail!(
                    "Path to include cannot be inside the src directory, use `--allow-include-src` to include files inside the src directory"
                );
            }

            log::debug!("Entry: {}", path.display());
            files.push(path);
        }
    }

    if files.is_empty() {
        log::info!("No files to process");
        return Ok(());
    }

    let mut pipelines = get_pipelines();
    let mut tasks = Vec::new();

    loop {
        if files.is_empty() {
            if !pipelines.is_empty() {
                let pipeline_names = pipelines.iter().map(|p| p.name()).collect::<Vec<_>>();
                log::info!(
                    "No more files to process, the next pipelines were not run: {}",
                    pipeline_names.join(", ")
                );
            }

            break;
        }

        let Some(pipeline) = pipelines.pop() else {
            break;
        };

        let mut target_files = vec![];
        let mut i = 0;

        while i < files.len() {
            if pipeline.can_process(&files[i], dest_dir) {
                let file = files.remove(i);
                target_files.push(file);
            } else {
                i += 1;
            }
        }

        tasks.push(async {
            let pipeline_name = pipeline.name().to_owned();
            pipeline
                .spawn(target_files, dest_dir)
                .await
                .with_context(|| format!("error processing `{pipeline_name}` pipeline"))
        });
    }

    let results = futures::future::join_all(tasks).await;
    for ret in results {
        if let Err(err) = ret {
            log::error!("{err}");
        }
    }

    Ok(())
}

fn is_outside_directory(base: &Path, path: &Path) -> anyhow::Result<bool> {
    let base_dir = base.canonicalize()?;
    let path_dir = path.canonicalize()?;

    match path_dir.strip_prefix(base_dir) {
        Ok(_) => Ok(false),
        Err(_) => Ok(true),
    }
}

fn is_inside_src(base: &Path, path: &Path) -> anyhow::Result<bool> {
    if !base.join("src").exists() {
        log::debug!("`src` directory not found");
        return Ok(false);
    }

    let base_dir = base.canonicalize()?;
    let path_dir = path.canonicalize()?;

    match path_dir.strip_prefix(base_dir) {
        Ok(remaining) => {
            if remaining.starts_with("src") {
                return Ok(true);
            }

            Ok(false)
        }
        Err(_) => Ok(false),
    }
}

// TODO: Should we just process the pipeline in order and forget about using a Box<dyn Pipeline>?
fn get_pipelines() -> Vec<Box<dyn Pipeline + Send>> {
    vec![
        // TODO: Add pipeline to process SCSS, SASS

        // Add any additional pipelines, all should be place before copy
        Box::new(CopyFilesPipeline),
    ]
}