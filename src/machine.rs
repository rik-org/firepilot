use std::{fs::copy, path::Path};

use crate::{
    builder::Configuration,
    executor::{Action, Executor},
};

#[derive(Debug)]
pub enum FirepilotError {
    Setup(String),
    Configure(String),
    Execute(String),
}

pub struct Machine {
    executor: Executor,
}

impl Machine {
    pub fn new() -> Self {
        Machine {
            executor: Executor::new(),
        }
    }

    pub fn copy<P, Q>(from: P, to: Q) -> Result<(), FirepilotError>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        copy(&from, &to).map_err(|e| {
            let msg = format!(
                "Failed to copy {:?} to {:?}: {}",
                from.as_ref(),
                to.as_ref(),
                e
            );
            FirepilotError::Setup(msg)
        })?;
        Ok(())
    }

    pub async fn create(&mut self, mut config: Configuration) -> Result<(), FirepilotError> {
        self.executor = config.executor.unwrap();
        // Step 1. Setup the machine workspace from the executor
        self.executor.create_workspace()?;

        // Step 3. Copy drives into the machine workspace
        let kernel = config.kernel.unwrap();
        for drive in config.storage.iter_mut() {
            let new_drive_path = self.executor.chroot().join(&drive.drive_id);
            Machine::copy(&drive.path_on_host, &new_drive_path)?;
            drive.path_on_host = new_drive_path.into_os_string().into_string().unwrap();
        }

        // Step 4. Copy the kernel in the system workspace
        Machine::copy(
            kernel.kernel_image_path.clone(),
            self.executor.chroot().join("vmlinux"),
        )?;

        if let Some(initrd) = kernel.initrd_path.clone() {
            Machine::copy(initrd, self.executor.chroot().join("initrd"))?;
        }

        // Step 5. Spawn the socket process
        self.executor.run_socket()?;

        // Step 6. Configure the socket with given informations from the configuration
        self.executor.configure_drives(config.storage).await?;
        self.executor.configure_boot_source(kernel).await?;
        self.executor.configure_network(config.interfaces).await?;
        Ok(())
    }

    /// Shutdown abruptly the socket process, if the VM was running it will stop it
    pub async fn kill(&mut self) -> Result<(), FirepilotError> {
        self.executor.destroy_socket().await?;
        Ok(())
    }

    /// Send a InstanceStart signal to the VM
    pub async fn start(&self) -> Result<(), FirepilotError> {
        self.executor.send_action(Action::InstanceStart).await?;
        Ok(())
    }

    /// Send a CtrlAltDel signal so it will shutdown gracefully
    pub async fn stop(&self) -> Result<(), FirepilotError> {
        self.executor.send_action(Action::SendCtrlAltDel).await?;
        Ok(())
    }
}
