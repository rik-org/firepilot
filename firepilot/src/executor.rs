//! # Low-Level Implementation of the microVM
//!
//! The executor is the component that will run the virtual machine. It is responsible for
//! starting the microVM and managing the socket that will be used to communicate with it.
//!
//! ## Design
//!
//! Executor implementation is a low level component which is not meant to be
//! used directly, except if you intend to have your wrapper around it. It gives
//! control on the full lifecycle of a microVM and actions that can be performed
//! on it. It is meant to give full control, compared to [Machine] which gives a
//! high-level API.
//!
//! ## Implementation
//!
//! You can either run firecracker directly with the binary by using
//! [FirecrackerExecutor] or you could decide to be safer and run with a
//! JailerExecutor. Be aware that the JailerExecutor is not yet implemented, but
//! we welcome contributions.
use std::{path::PathBuf, process::Stdio};

use tokio::process::{Child, Command};

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};
use tracing::{debug, error, info, instrument, trace};

use crate::machine::FirepilotError;
use firecracker_models::models::{BootSource, Drive, NetworkInterface};

/// Interface to determine how to execute commands on the socket and where to do it
pub trait Execute {
    /// Define where all the drives, rootfs, kernel and socket will be created
    fn chroot(&self) -> PathBuf;
    /// Execute a command onto the binary behind the executor
    ///
    /// It is only used to spawn the executor process, not to send commands to it
    fn spawn_binary_child(&self, args: &Vec<String>) -> Result<Child, ExecuteError>;
}

#[derive(thiserror::Error, Debug)]
pub enum ExecuteError {
    #[error("Could not initate worksapce for machine, reason: {0}")]
    WorkspaceCreation(String),
    #[error("Could not delete worksapce for machine, reason: {0}")]
    WorkspaceDeletion(String),
    #[error("Could not execute command, reason: {0}")]
    CommandExecution(String),
    #[error("Failed to manage socket, reason: {0}")]
    Socket(String),
    #[error("Could not send request on uri {0}, reason: {1}")]
    Request(hyper::Uri, String),
    #[error("Could not serialize request, reason: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Socket didn't start on time")]
    Unhealthy,
}

impl From<ExecuteError> for FirepilotError {
    fn from(e: ExecuteError) -> FirepilotError {
        match e {
            ExecuteError::CommandExecution(e) => FirepilotError::Setup(e),
            ExecuteError::Request(url, e) => FirepilotError::Configure(format!("{}: {}", url, e)),
            ExecuteError::Serialize(e) => FirepilotError::Configure(e.to_string()),
            ExecuteError::Socket(e) => FirepilotError::Configure(e),
            ExecuteError::WorkspaceCreation(e) => FirepilotError::Setup(e),
            ExecuteError::WorkspaceDeletion(e) => FirepilotError::Setup(e),
            ExecuteError::Unhealthy => {
                FirepilotError::Configure("Socket didn't start on time".to_string())
            }
        }
    }
}

/// Action available on the VM
#[derive(Debug, Serialize)]
#[serde(tag = "action_type", rename_all = "PascalCase")]
pub enum Action {
    InstanceStart,
    SendCtrlAltDel,
}

/// Contains an instance of the microVM, this low-level implementation hold the
/// process and is able to talk to the socket in order to configure the microVM.
#[derive(Debug)]
pub struct Executor {
    /// Optional executor, if none is provided, it will crash as no other
    /// executor is available
    ///
    /// It is not a [Box<dyn Execute>] because we didn't want to use Boxes
    /// everywhere. We could have been using an enum, but due to the small
    /// number of implementation we judged it was not worth it.
    firecracker: Option<FirecrackerExecutor>,
    /// Holds the process of the executor when it is running
    socket_process: Option<Child>,
    /// A RPC client to talk to the socket
    client: Client<UnixConnector>,
    /// ID given when creating the executor, it doesn't need to be unique, but
    /// we really encourage to make it unique and it might collapse if you run
    /// two VM with the same ID at the same time (file system issues).
    id: String,
}

impl Executor {
    /// Create a new Executor with no implementation, and with id "default"
    pub fn new() -> Executor {
        Executor {
            firecracker: None,
            socket_process: None,
            id: "default".to_string(),
            client: Client::unix(),
        }
    }
    /// Create a new Executor with the firecracker binary
    pub fn new_with_firecracker(firecracker: FirecrackerExecutor) -> Executor {
        Executor {
            firecracker: Some(firecracker),
            socket_process: None,
            id: "default".to_string(),
            client: Client::unix(),
        }
    }

    /// Mutate the executor to have a new id
    pub fn with_id(self, id: String) -> Executor {
        Executor { id, ..self }
    }

    /// Tells whether the mVM is running or not
    pub fn is_running(&self) -> bool {
        self.socket_process.is_some()
    }

    /// Return the configured executor, or panic if none is configured
    fn executor(&self) -> &dyn Execute {
        match &self.firecracker {
            Some(firecracker) => return firecracker,
            None => panic!("No executor found"),
        }
    }

    #[instrument(skip(self), fields(id = %self.id))]
    fn wait_healthy(&self) -> Result<(), ExecuteError> {
        debug!("Waiting for socket to be healthy");
        let sock = self.chroot().join("firecracker.socket");
        let mut retries = 0;
        while retries < 10 {
            let res = std::fs::metadata(&sock);
            if res.is_ok() {
                debug!("Socket is now healthy");
                return Ok(());
            }
            retries += 1;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        debug!("Socket is not healthy");
        Err(ExecuteError::Unhealthy)
    }

    #[instrument(skip_all, fields(id = %self.id))]
    async fn send_request(&self, url: hyper::Uri, body: String) -> Result<(), ExecuteError> {
        debug!("Send request to socket: {}", url);
        trace!("Sent body to socket [{}]: {}", url, body);
        let request = Request::builder()
            .method(Method::PUT)
            .uri(url.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(Body::from(body))
            .map_err(|e| ExecuteError::Request(url.clone(), e.to_string()))?;

        let response = self
            .client
            .request(request)
            .await
            .map_err(|e| ExecuteError::Request(url.clone(), e.to_string()))?;

        trace!("Response status: {:#?}", response.status());
        let status = response.status();
        if !status.is_success() {
            error!("Request to socket failed [{}]: {:#?}", url, status);
            // body stream to string
            let body = hyper::body::to_bytes(response.into_body())
                .await
                .map_err(|e| ExecuteError::Request(url.clone(), e.to_string()))?;
            error!(
                "Request [{}] body: {}",
                url,
                String::from_utf8(body.to_vec()).unwrap()
            );
            return Err(ExecuteError::CommandExecution(format!(
                "Failed to send request to {}, status: {}",
                url, status
            )));
        }

        Ok(())
    }

    /// Sends a specific [Action] to the microVM
    #[instrument(skip_all, fields(id = %self.id))]
    pub async fn send_action(&self, action: Action) -> Result<(), ExecuteError> {
        debug!("Send action to socket: {:#?}", action);
        let json = serde_json::to_string(&action).map_err(ExecuteError::Serialize)?;

        let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), "/actions").into();
        self.send_request(url, json).await?;
        Ok(())
    }

    /// Full path to the chroot of the machine which contains the socket, drives, kernel, etc...
    pub fn chroot(&self) -> PathBuf {
        self.executor().chroot().join(&self.id)
    }

    /// Tries to spawn the executor process, the workspace for the machine should
    /// already exist ([create_workspace] should have been called)
    #[instrument(skip(self), fields(id = %self.id))]
    pub fn run_socket(&mut self) -> Result<(), ExecuteError> {
        info!("Running the socket");
        let executor = self.executor();
        let sock = self.chroot().join("firecracker.socket");

        let child = executor.spawn_binary_child(&vec![
            "--api-sock".to_string(),
            sock.into_os_string().into_string().unwrap(),
        ])?;
        self.wait_healthy()?;
        self.socket_process = Some(child);
        debug!("Socket is now running");
        Ok(())
    }

    /// Shutdown abruptly the socket process, if the VM was running it will stop it
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn destroy_socket(&mut self) -> Result<(), ExecuteError> {
        info!("Destroying the socket");
        let sock_path = self.chroot().join("firecracker.socket");

        let socket = self.socket_process.as_mut().ok_or_else(|| {
            ExecuteError::Socket(
                "Socket hasn't been spawned, you must spawn it before destroying it".to_string(),
            )
        })?;
        socket
            .kill()
            .await
            .map_err(|e| ExecuteError::Socket(e.to_string()))?;
        std::fs::remove_file(sock_path).map_err(|e| ExecuteError::Socket(e.to_string()))?;
        debug!("Socket is now destroyed and the socket file doesn't exist anymore");
        self.socket_process = None;
        Ok(())
    }

    /// Apply the boot source configuration to the VM
    #[instrument(skip_all, fields(id = %self.id))]
    pub async fn configure_boot_source(&self, boot_source: BootSource) -> Result<(), ExecuteError> {
        debug!("Configure boot source");
        trace!("Boot source: {:#?}", boot_source);
        let json = serde_json::to_string(&boot_source).map_err(ExecuteError::Serialize)?;

        let url: hyper::Uri =
            Uri::new(self.chroot().join("firecracker.socket"), "/boot-source").into();
        self.send_request(url, json).await?;
        Ok(())
    }

    /// Apply all drives configuration on the VM
    #[instrument(skip_all, fields(id = %self.id))]
    pub async fn configure_drives(&self, drives: Vec<Drive>) -> Result<(), ExecuteError> {
        debug!("Configure drives");
        for drive in drives {
            debug!("Configure drive {}", drive.drive_id);
            trace!("Drive: {:#?}", drive);
            let json = serde_json::to_string(&drive).map_err(ExecuteError::Serialize)?;

            let path = format!("/drives/{}", drive.drive_id);
            let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), &path).into();
            self.send_request(url, json).await?;
        }
        Ok(())
    }

    /// Apply network configuration on the VM
    #[instrument(skip_all, fields(id = %self.id))]
    pub async fn configure_network(
        &self,
        network_interfaces: Vec<NetworkInterface>,
    ) -> Result<(), ExecuteError> {
        debug!("Configure network interfaces");
        for network_interface in network_interfaces {
            debug!("Configure network interface {}", network_interface.iface_id);
            trace!("Network interface: {:#?}", network_interface);
            let json =
                serde_json::to_string(&network_interface).map_err(ExecuteError::Serialize)?;

            let path = format!("/network-interfaces/{}", network_interface.iface_id);
            let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), &path).into();
            self.send_request(url, json).await?;
        }
        Ok(())
    }

    /// Create needed folders where the VM will be configured
    #[instrument(skip(self), fields(id = %self.id))]
    pub fn create_workspace(&self) -> Result<(), ExecuteError> {
        debug!("Creating workspace at {}", self.chroot().display());
        std::fs::create_dir_all(self.chroot())
            .map_err(|e| ExecuteError::WorkspaceCreation(e.to_string()))?;
        Ok(())
    }
}

/// Implementation of Executor for Firecracker, it will spawn the microVM using
/// firecracker binary
#[derive(Debug)]
pub struct FirecrackerExecutor {
    /// Path to a folder where all files related to the microVM will be stored,
    /// it is used by higher level abstractions to store drives, kernel, etc...
    pub chroot: String,
    /// Path to the firecracker binary
    pub exec_binary: PathBuf,
}

impl Execute for FirecrackerExecutor {
    fn chroot(&self) -> PathBuf {
        PathBuf::from(&self.chroot)
    }

    fn spawn_binary_child(&self, args: &Vec<String>) -> Result<Child, ExecuteError> {
        let command = Command::new(&self.exec_binary)
            .args(args)
            // FIXME: Implement logging
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ExecuteError::CommandExecution(e.to_string()))?;
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    #[tokio::test]
    async fn test_executor() {
        let executor = FirecrackerExecutor {
            chroot: "/tmp/firepilot".to_string(),
            exec_binary: PathBuf::from("/usr/bin/firecracker"),
        };
        let mut machine = Executor::new_with_firecracker(executor);
        machine.create_workspace().unwrap();
        machine.run_socket().expect("Failed to run socket");

        // expect socket to exist
        let socket = machine.chroot().join("firecracker.socket");
        assert!(socket.exists());

        machine.destroy_socket().await.expect("fail to kill");
        assert!(!socket.exists());
    }

    #[tokio::test]
    #[should_panic]
    async fn test_destroy_when_no_init() {
        let executor = FirecrackerExecutor {
            chroot: "/tmp/firepilot2".to_string(),
            exec_binary: PathBuf::from("/usr/bin/firecracker"),
        };
        let mut machine = Executor::new_with_firecracker(executor);
        machine.create_workspace().unwrap();
        machine.destroy_socket().await.expect("fail to kill");
    }

    #[test]
    #[should_panic]
    fn test_no_executor_fails() {
        let machine = Executor {
            firecracker: None,
            socket_process: None,
            id: "default".to_string(),
            client: Client::unix(),
        };
        machine.create_workspace().unwrap();
    }
}
