use std::path::PathBuf;

use tokio::process::{Child, Command};

use hyper::{Body, Client, Method, Request};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

use crate::machine::FirepilotError;
use crate::models::{BootSource, Drive, NetworkInterface};

pub trait Execute {
    fn chroot(&self) -> PathBuf;
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

#[derive(Debug, Serialize)]
#[serde(tag = "action_type", rename_all = "PascalCase")]
pub enum Action {
    InstanceStart,
    SendCtrlAltDel,
}

pub struct Executor {
    firecracker: Option<FirecrackerExecutor>,
    socket_process: Option<Child>,
    client: Client<UnixConnector>,
    id: String,
}

impl Executor {
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

    fn wait_healthy(&self) -> Result<(), ExecuteError> {
        let sock = self.chroot().join("firecracker.socket");
        let mut retries = 0;
        while retries < 10 {
            let res = std::fs::metadata(&sock);
            if res.is_ok() {
                return Ok(());
            }
            retries += 1;
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Err(ExecuteError::Unhealthy)
    }

    async fn send_request(&self, url: hyper::Uri, body: String) -> Result<(), ExecuteError> {
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

        let status = response.status();
        if !status.is_success() {
            return Err(ExecuteError::CommandExecution(format!(
                "Failed to send request to {}, status: {}",
                url, status
            )));
        }

        Ok(())
    }

    pub async fn send_action(&self, action: Action) -> Result<(), ExecuteError> {
        let json = serde_json::to_string(&action).map_err(ExecuteError::Serialize)?;

        let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), "/actions").into();
        println!("Sending request to {}, boy: {}", &url, &json);
        self.send_request(url, json).await?;
        Ok(())
    }

    /// Full path to the chroot of the machine which contains the socket, drives, kernel, etc...
    pub fn chroot(&self) -> PathBuf {
        self.executor().chroot().join(&self.id)
    }

    /// Tries to spawn the executor process, the workspace for the machine should
    /// already exist ([Executor::create_workspace] should have been called)
    pub fn run_socket(&mut self) -> Result<(), ExecuteError> {
        let executor = self.executor();
        let sock = self.chroot().join("firecracker.socket");

        let child = executor.spawn_binary_child(&vec![
            "--api-sock".to_string(),
            sock.into_os_string().into_string().unwrap(),
        ])?;
        self.wait_healthy()?;
        self.socket_process = Some(child);
        Ok(())
    }

    /// Shutdown abruptly the socket process, if the VM was running it will stop it
    pub async fn destroy_socket(&mut self) -> Result<(), ExecuteError> {
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

        self.socket_process = None;
        Ok(())
    }

    /// Apply the boot source configuration to the VM
    pub async fn configure_boot_source(&self, boot_source: BootSource) -> Result<(), ExecuteError> {
        let json = serde_json::to_string(&boot_source).map_err(ExecuteError::Serialize)?;

        let url: hyper::Uri =
            Uri::new(self.chroot().join("firecracker.socket"), "/boot-source").into();
        self.send_request(url, json).await?;
        Ok(())
    }

    /// Apply all drives configuration on the VM
    pub async fn configure_drives(&self, drives: Vec<Drive>) -> Result<(), ExecuteError> {
        for drive in drives {
            let json = serde_json::to_string(&drive).map_err(ExecuteError::Serialize)?;

            let path = format!("/drives/{}", drive.drive_id);
            let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), &path).into();
            self.send_request(url, json).await?;
        }
        Ok(())
    }

    /// Apply network configuration on the VM
    pub async fn configure_network(
        &self,
        network_interfaces: Vec<NetworkInterface>,
    ) -> Result<(), ExecuteError> {
        for network_interface in network_interfaces {
            let json =
                serde_json::to_string(&network_interface).map_err(ExecuteError::Serialize)?;

            let path = format!("/network-interfaces/{}", network_interface.iface_id);
            let url: hyper::Uri = Uri::new(self.chroot().join("firecracker.socket"), &path).into();
            self.send_request(url, json).await?;
        }
        Ok(())
    }

    /// Create needed folders where the VM will be configured
    pub fn create_workspace(&self) -> Result<(), ExecuteError> {
        std::fs::create_dir_all(self.chroot())
            .map_err(|e| ExecuteError::WorkspaceCreation(e.to_string()))?;
        Ok(())
    }
}

pub struct FirecrackerExecutor {
    pub chroot: String,
    pub exec_binary: PathBuf,
}

impl Execute for FirecrackerExecutor {
    fn chroot(&self) -> PathBuf {
        PathBuf::from(&self.chroot)
    }

    fn spawn_binary_child(&self, args: &Vec<String>) -> Result<Child, ExecuteError> {
        let command = Command::new(&self.exec_binary)
            .args(args)
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
        machine.run_socket().unwrap();

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
        machine.create_workspace();
    }
}
