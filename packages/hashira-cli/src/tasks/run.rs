use super::build::BuildTask;
use crate::{
    cli::{BuildOptions, RunOptions},
    utils::wait_interruptible,
};
use anyhow::Context;
use std::{collections::HashMap, path::PathBuf};
use tokio::{
    process::{Child, Command},
    sync::broadcast::Sender,
};

pub struct RunTask {
    // Options for running the application
    pub options: RunOptions,

    // Additional environment variables
    pub envs: HashMap<&'static str, String>,

    // A receiver for shutdown the executing process
    pub interrupt_signal: Option<Sender<()>>,

    // Notify when a build is done
    pub build_done_signal: Option<Sender<()>>,
}

impl RunTask {
    pub fn new(options: RunOptions) -> Self {
        RunTask {
            options,
            envs: Default::default(),
            interrupt_signal: None,
            build_done_signal: None,
        }
    }

    pub fn with_signal(
        options: RunOptions,
        shutdown_signal: Sender<()>,
        build_done_signal: Sender<()>,
    ) -> Self {
        RunTask {
            options,
            envs: Default::default(),
            interrupt_signal: Some(shutdown_signal),
            build_done_signal: Some(build_done_signal),
        }
    }

    pub fn env(&mut self, name: &'static str, value: String) {
        self.envs.insert(name, value);
    }

    pub async fn run(self) -> anyhow::Result<()> {
        // Builds the app
        self.build().await?;

        // Run the generated exe
        self.exec().await?;

        Ok(())
    }

    async fn build(&self) -> anyhow::Result<()> {
        let opts = &self.options;
        let build_done_signal = &self.build_done_signal;
        let build_task = BuildTask {
            options: BuildOptions::from(opts),
            interrupt_signal: self.interrupt_signal.clone(),
        };

        build_task.run().await?;

        if let Some(build_done_signal) = build_done_signal {
            //let _ = build_done_signal.send(());
            if let Err(err) = build_done_signal.send(()) {
                log::error!("Error sending build done signal: {err}");
            }
        }

        Ok(())
    }

    async fn exec(&self) -> anyhow::Result<()> {
        let spawn = self.spawn_server_exec()?;
        wait_interruptible(spawn, self.interrupt_signal.clone())
            .await
            .context("failed to run")?;
        Ok(())
    }

    fn spawn_server_exec(&self) -> anyhow::Result<Child> {
        let opts = &self.options;
        let exec_path = self
            .get_executable_path()
            .context("Failed to get executable path")?;

        log::debug!("Executable path: {}", exec_path.display());

        let mut cmd = Command::new(exec_path);
        let wasm_lib = crate::utils::get_cargo_lib_name()?;

        // environment variables
        log::debug!("host: {}", opts.host);
        log::debug!("port: {}", opts.port);
        log::debug!("static files: {}", opts.static_dir);

        cmd.env(crate::env::HASHIRA_HOST, &opts.host);
        cmd.env(crate::env::HASHIRA_PORT, opts.port.to_string());
        cmd.env(crate::env::HASHIRA_STATIC_DIR, &opts.static_dir);
        cmd.env(crate::env::HASHIRA_WASM_LIB, wasm_lib);

        for (name, value) in self.envs.iter() {
            cmd.env(name, value);
        }

        let child = cmd.spawn()?;
        Ok(child)
    }

    fn get_executable_path(&self) -> anyhow::Result<PathBuf> {
        let opts = &self.options;
        let exec_name = crate::utils::get_exec_name()?;
        let target_dir = opts.profile_target_dir()?;
        let exec_path = target_dir.join(format!("{exec_name}.exe"));
        Ok(exec_path)
    }
}
