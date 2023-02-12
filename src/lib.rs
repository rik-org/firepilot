use std::{
    env::{split_paths, var_os},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use exec::{Args, Executable};
use log::debug;
use microvm::MicroVM;

mod exec;
mod microvm;

#[derive(Debug, thiserror::Error)]
pub enum FirecrackerError {
    #[error("Unable to find the firecracker binary on host")]
    BinaryNotFound,
    #[error("Unable to create the firecracker working directory : {0:?}")]
    WorkingDirCreation(std::io::Error),
    #[error("Failed to spawn firecracker command: {0:?}")]
    ProcessSpawn(std::io::Error),
    #[error("Failed to execute firecracker command: {0:?}")]
    Exec(std::io::Error),
    #[error("firecracker command has failed. stdout: {0:?}, stderr: {1:?}")]
    CommandFailed(String, String),
}

type Result<T, E = FirecrackerError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct FirecrackerOptions {
    /// Path to the `firecracker` binary on host.
    /// If not set, the initialization will try to
    /// retrieve it from the `$PATH`.
    pub command: Option<PathBuf>,
    /// Path to a directory where to store `firecracker` related files
    /// such as sockets, VM configuration etc.
    pub working_dir: Option<PathBuf>,
}

impl Default for FirecrackerOptions {
    fn default() -> Self {
        Self {
            command: None,
            working_dir: Some(PathBuf::from("/tmp/firecracker")),
        }
    }
}

pub struct Firecracker {
    /// Path to the `firecracker` binary on host.
    command: PathBuf,
    /// Path to the working directory for `firecracker`.
    working_dir: PathBuf,
}

impl Firecracker {
    /// Instanciate a new `Firecracker` instance with the given options.
    pub fn new(opts: Option<FirecrackerOptions>) -> Result<Self> {
        let options = opts.unwrap_or_default();

        let command = options.command.or_else(|| {
            // Check in `$PATH` for the firecracker binary location
            var_os("PATH").and_then(|paths| {
                split_paths(&paths)
                    .filter_map(|d| {
                        let full_path = d.join("firecracker");
                        if full_path.is_file() {
                            Some(full_path)
                        } else {
                            None
                        }
                    })
                    .next()
            })
        });

        if command.is_none() || !command.clone().unwrap().exists() {
            return Err(FirecrackerError::BinaryNotFound);
        }

        let working_dir = options.working_dir.unwrap();
        std::fs::create_dir_all(&working_dir).map_err(FirecrackerError::WorkingDirCreation)?;

        Ok(Self {
            command: command.unwrap(),
            working_dir,
        })
    }

    /// Start the given microVM on the host.
    pub fn start(&self, vm: &MicroVM) -> Result<String> {
        // Compute the VM socket path inside the active working directory.
        let sock = self.vm_socket_path(&vm.id);

        // Compute the VM configuration path inside the active working directory.
        let cfg_file = self
            .working_dir
            .join(format!("vm-{}.cfg.json", vm.id))
            .display()
            .to_string();

        // Serialize the VM configuration and write it to the configuration file.
        let ser = serde_json::to_string(&vm.config).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&cfg_file)
            .unwrap();

        file.write_all(ser.as_bytes()).unwrap();

        self.exec(&vec![
            String::from("--api-sock"),
            sock,
            String::from("--config-file"),
            cfg_file,
        ])
    }

    // Compute a path to the socket corresponding to the given VM identifier inside
    // the current working directory.
    fn vm_socket_path(&self, vm_id: &String) -> String {
        self.working_dir
            .join(format!("vm-{}.sock", vm_id))
            .display()
            .to_string()
    }
}

impl Args for Firecracker {
    fn args(&self) -> Result<Vec<String>> {
        Ok(Vec::<String>::new())
    }
}

impl Executable for Firecracker {
    fn exec(&self, args: &Vec<String>) -> Result<String> {
        let args = self.concat_args(args)?;

        debug!("{} {}", self.command.display(), args.join(" "));

        let process = Command::new(&self.command)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(FirecrackerError::Exec)?;

        let result = process.wait_with_output().unwrap();
        let stdout = String::from_utf8(result.stdout).unwrap();
        let stderr = String::from_utf8(result.stderr).unwrap();

        if !result.status.success() {
            if !stderr.is_empty() {
                log::error!("firecracker error: {}", stderr)
            }
            return Err(FirecrackerError::CommandFailed(stdout, stderr));
        }

        Ok(stdout)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::microvm::{BootSource, Config, Drive, MicroVM, NetworkInterface};
    use crate::{Firecracker, FirecrackerOptions};

    #[test]
    fn test_can_instantiate_firecracker_from_path() {
        let firecracker = Firecracker::new(None);
        assert!(firecracker.is_ok())
    }

    #[test]
    fn test_can_instantiate_firecracker_from_custom_path() {
        let firecracker = Firecracker::new(Some(FirecrackerOptions {
            command: Some(PathBuf::from("./fixtures/firecracker")),
            ..FirecrackerOptions::default()
        }));
        assert!(firecracker.is_ok())
    }

    #[test]
    fn test_cannot_instantiate_firecracker_if_binary_not_found() {
        let firecracker = Firecracker::new(Some(FirecrackerOptions {
            command: Some(PathBuf::from("/randomdir/firecracker")),
            ..FirecrackerOptions::default()
        }));
        assert!(firecracker.is_err())
    }

    #[test]
    fn test_it_run_vm_from_config() {
        let firecracker = Firecracker::new(None).unwrap();
        let vm = MicroVM::from(Config {
            boot_source: BootSource {
                kernel_image_path: PathBuf::from(
                    "/home/debian/developer/firepilot/fixtures/hello-vmlinux.bin",
                ),
                boot_args: None,
                initrd_path: None,
            },
            drives: vec![Drive {
                drive_id: "rootfs".to_string(),
                path_on_host: PathBuf::from(
                    "/home/debian/developer/firepilot/fixtures/rootfs.ext4",
                ),
                is_read_only: false,
                is_root_device: true,
            }],
            network_interfaces: vec![NetworkInterface {
                iface_id: "eth0".to_string(),
                guest_mac: Some("AA:FC:00:00:00:01".to_string()),
                host_dev_name: "tap0".to_string(),
            }],
        });

        firecracker.start(&vm).unwrap();
    }
}
