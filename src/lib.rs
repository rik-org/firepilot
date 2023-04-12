use std::{
    env::{split_paths, var_os},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use exec::{Args, Executable};
use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, Uri};
use log::debug;
use microvm::MicroVM;
use serde_json::json;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
extern crate url;

mod exec;
pub mod microvm;
pub mod models;
pub mod builder;
pub mod executor;
pub mod machine;

const DEFAULT_WORKING_DIR: &str = "/tmp/firecracker";

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
    #[error("Failed to create a request: {0:?}")]
    RequestBuilderFailed(hyper::http::Error),
    #[error("Failed to execute request: {0:?}")]
    RequestFailed(hyper::Error),
}

type Result<T, E = FirecrackerError> = std::result::Result<T, E>;

#[derive(Debug, Default)]
pub struct FirecrackerOptions {
    /// Path to the `firecracker` binary on host, you can provide one via other means
    /// (see [Firecracker::new])
    pub command: Option<PathBuf>,
    /// Path to a directory where to store `firecracker` related files
    /// such as sockets, VM configuration etc.
    pub working_dir: Option<PathBuf>,
    /// If [`working_dir`](FirecrackerOptions) is provided, this flag will instruct to create the path to the working
    /// directory if it doesn't exist.
    pub create_working_dir: bool,
}

pub struct Firecracker {
    /// Path to the `firecracker` binary on host.
    binary_path: PathBuf,
    /// Path to the working directory for `firecracker`.
    working_dir: PathBuf,
}

impl Default for Firecracker {
    /// Create a new firecracker interface, it will try to determine the binary location (you can customize
    /// the location, see [Firecracker::new])
    fn default() -> Self {
        let binary =
            Firecracker::determine_binary_location().expect("Unable to find firecracker binary");
        let working_dir = PathBuf::from(DEFAULT_WORKING_DIR);
        Firecracker::create_working_dir(&working_dir)
            .expect("Unable to create a directory to store sockets");
        Self {
            binary_path: binary,
            working_dir: working_dir,
        }
    }
}

impl Firecracker {
    /// Tries to determine if `firecracker` binary exists in the `$PATH` variable, if it does, it will
    /// return the path to the binary.
    fn find_binary_from_path() -> Option<PathBuf> {
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
    }

    /// Tries to determine if `firecracker` binary exists in the current working directory, if it does,
    /// it will return the path to the binary.
    fn find_binary_from_current_directory() -> Option<PathBuf> {
        let full_path = PathBuf::from("./firecracker");
        match full_path.is_file() {
            true => Some(full_path),
            false => None,
        }
    }

    /// Tries to determine if variable `FIRECRACKER_LOCATION` exists, if it does, it will check if
    /// firecracker binary exists, if it does, it will return the content of the variable.
    fn find_binary_from_env_location() -> Option<PathBuf> {
        if let Some(path) = var_os("FIRECRACKER_LOCATION") {
            if PathBuf::from(&path).is_file() {
                return Some(PathBuf::from(path));
            }

            log::warn!(
                "FIRECRACKER_LOCATION is set but the file does not exist: {:?}",
                path
            );
        }
        None
    }
    /// Tries to determine `firecracker` binary location, in case it cannot determine any binary it
    /// will panic
    ///
    /// It is based on multiple sources (top to bottom priority).
    ///
    /// - `FIRECRACKER_LOCATION` environment variable: direct path to the binary
    /// - `$PATH` environment variable: search for the binary in the directories
    /// - `firecracker` binary in the current working directory
    pub fn determine_binary_location() -> Result<PathBuf> {
        Self::find_binary_from_env_location()
            .or_else(Self::find_binary_from_path)
            .or_else(Self::find_binary_from_current_directory)
            .map(|p| Ok(p))
            .unwrap_or(Err(FirecrackerError::BinaryNotFound))
    }

    fn create_working_dir(working_dir: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(&working_dir).map_err(FirecrackerError::WorkingDirCreation)
    }

    /// Create a new firecracker interface, it will try to determine the binary location, but you can
    /// provide a custom one through several options (upper take priority over lower):
    ///
    /// - `command` field in the `FirecrackerOptions` structure
    /// - `FIRECRACKER_LOCATION` environment variable: direct path to the binary
    /// - `$PATH` environment variable: search for the binary in the directories
    /// - `firecracker` binary in the current working directory
    ///
    /// If you provided a custom path to the binary and the binary doesn't exist it will return
    /// [FirecrackerError::BinaryNotFound].
    ///
    /// If you don't provide a directory to store `firecracker` related files, it will use the
    /// default one ([DEFAULT_WORKING_DIR]).
    pub fn new(options: FirecrackerOptions) -> Result<Self> {
        println!("{:?}", options);
        let binary_path = match options.command {
            Some(path) => Ok(path),
            None => Self::determine_binary_location(),
        }?;
        let working_dir = options.working_dir.unwrap_or_default();

        if options.create_working_dir {
            Self::create_working_dir(&working_dir)?;
        }

        Ok(Self {
            binary_path,
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

    pub async fn stop(&self, vm: &MicroVM) -> Result<()> {
        let sock = self.vm_socket_path(&vm.id);
        let url: Uri = Uri::new(sock, "/actions").into();

        let client = Client::unix();
        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .body(Body::from(
                json!({ "action_type": "SendCtrlAltDel" }).to_string(),
            ))
            .map_err(FirecrackerError::RequestBuilderFailed)?;

        client
            .request(req)
            .await
            .map_err(FirecrackerError::RequestFailed)?;
        Ok(())
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

        debug!("{} {}", self.binary_path.display(), args.join(" "));

        let process = Command::new(&self.binary_path)
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
    use std::env::var_os;
    use std::fs::File;

    use tempfile::tempdir;

    use crate::{Firecracker, FirecrackerOptions};

    #[test]
    fn test_can_determine_binary_location_from_env() {
        let dir = tempdir().expect("failed to create temporary directory");
        let file_path = dir.path().join("firecracker");
        let _file = File::create(file_path.clone()).expect("failed to create temporary file");
        std::env::set_var("FIRECRACKER_LOCATION", file_path);
        let result = Firecracker::determine_binary_location();
        assert!(result.is_ok())
    }

    #[test]
    fn test_cant_determine_binary_location_from_env() {
        std::env::set_var("FIRECRACKER_LOCATION", "/tmp/invalid_path/firecracker");
        let result = Firecracker::determine_binary_location();
        assert!(result.is_err());
    }

    #[test]
    fn test_can_determine_binary_location_from_path() {
        let dir = tempdir().expect("failed to create temporary directory");
        let file_path = dir.path().join("firecracker");
        let _file = File::create(file_path.clone()).expect("failed to create temporary file");

        std::env::set_var("PATH", file_path.parent().unwrap());
        println!("{:?}", var_os("PATH"));
        let result = Firecracker::determine_binary_location();
        assert!(result.is_ok())
    }

    #[test]
    fn test_cant_determine_binary_location_from_path() {
        std::env::set_var("PATH", "/tmp/invalid_path");
        let result = Firecracker::determine_binary_location();
        assert!(result.is_err())
    }

    #[test]
    fn test_can_determine_binary_location_from_option() {
        let dir = tempdir().expect("failed to create temporary directory");
        let file_path = dir.path().join("firecracker");
        let _file = File::create(file_path.clone()).expect("failed to create temporary file");
        let result = Firecracker::new(FirecrackerOptions {
            command: Some(file_path),
            ..Default::default()
        });
        assert!(result.is_ok())
    }
}
