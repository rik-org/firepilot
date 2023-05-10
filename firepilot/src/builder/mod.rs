//! # MicroVM Builder Pattern
//!
//! This module provides a builder pattern to make the configuration of microVM
//! easier. For each component, all fields are optional and are validated once
//! you run the [`Builder::try_build`] method. Once the build is successful, you can
//! consider it will properly be configured and can be used to start a microVM.
//!
//! ## Example
//!
//! ```rust
//! use std::{
//!   fs::File,
//!   io::copy,
//!   path::{Path, PathBuf},
//! };
//! use firepilot_models::models::{BootSource, Drive, NetworkInterface};
//! use firepilot::builder::{Configuration, Builder};
//! use firepilot::builder::{drive::DriveBuilder, kernel::KernelBuilder};
//! use firepilot::builder::executor::FirecrackerExecutorBuilder;
//! let path = Path::new("examples/resources");
//! let kernel_path = path.join("kernel.bin");
//! let rootfs_path = path.join("rootfs.ext4");
//!
//! // Configure the kernel in the micro VM
//! let kernel = KernelBuilder::new()
//!     .with_kernel_image_path(kernel_path.to_str().unwrap().to_string())
//!     .with_boot_args("reboot=k panic=1 pci=off".to_string())
//!     .try_build()
//!     .unwrap();
//! // Create a single drive that will be used as rootfs
//! let drive = DriveBuilder::new()
//!     .with_drive_id("rootfs".to_string())
//!     .with_path_on_host(rootfs_path)
//!     .as_read_only()
//!     .as_root_device()
//!     .try_build()
//!     .unwrap();
//! // Configure the executor that will be used to start the microVM
//! // only firecracker is available, but you could add a jailer executor
//! let executor = FirecrackerExecutorBuilder::new()
//!     .with_chroot("./examples/executor/".to_string())
//!     .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
//!     .try_build()
//!     .unwrap();
//! // Execute the builder pattern to create the configuration which can be used
//! // to create a [Machine]
//! let config = Configuration::new("simple_vm".to_string())
//!     .with_kernel(kernel)
//!     .with_executor(executor)
//!     .with_drive(drive);
//! ```
use crate::executor::Executor;

use firepilot_models::models::{BootSource, Drive, NetworkInterface};

pub mod drive;
pub mod executor;
pub mod kernel;
pub mod network_interface;

fn assert_not_none<T>(key: &str, value: &Option<T>) -> Result<(), BuilderError> {
    match value {
        Some(_) => Ok(()),
        None => return Err(BuilderError::MissingRequiredField(key.to_string())),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuilderError {
    /// The field is required but was not provided in the builder object
    MissingRequiredField(String),
    /// Happens when using auto methods to detect firecracker /jailer binary
    BinaryNotFound(String),
}

/// Generic trait which all builder componenet must implement in order to be
/// part of [Configuration]
pub trait Builder<T> {
    /// Validate all the fields from the builder object and apply it to the
    /// final object
    ///
    /// ## Example
    ///
    /// ```rust
    /// use firepilot::builder::Builder;
    /// use firepilot::builder::network_interface::NetworkInterfaceBuilder;
    ///
    /// NetworkInterfaceBuilder::new()
    ///     .with_iface_id("eth0".to_string())
    ///     .with_host_dev_name("tap0".to_string())
    ///     .try_build()
    ///     .unwrap();
    /// ```
    fn try_build(self) -> Result<T, BuilderError>;
}

/// Configuration object which represent a microVM configuration, when using the
/// [Builder] the final object is this one.
#[derive(Debug)]
pub struct Configuration {
    pub executor: Option<Executor>,
    pub kernel: Option<BootSource>,
    pub storage: Vec<Drive>,
    pub interfaces: Vec<NetworkInterface>,

    pub vm_id: String,
}

impl Configuration {
    pub fn new(vm_id: String) -> Configuration {
        Configuration {
            kernel: None,
            executor: None,
            storage: Vec::new(),
            interfaces: Vec::new(),
            vm_id,
        }
    }

    pub fn with_kernel(mut self, kernel: BootSource) -> Configuration {
        self.kernel = Some(kernel);
        self
    }

    pub fn with_executor(mut self, executor: Executor) -> Configuration {
        let executor = executor.with_id(self.vm_id.clone());
        self.executor = Some(executor);
        self
    }

    pub fn with_drive(mut self, drive: Drive) -> Configuration {
        self.storage.push(drive);
        self
    }

    pub fn with_interface(mut self, iface: NetworkInterface) -> Configuration {
        self.interfaces.push(iface);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::{assert_not_none, BuilderError};

    #[test]
    fn macro_assert_not_none() {
        let x = Some(1);
        let y: Option<String> = None;
        assert_eq!(assert_not_none("x", &x), Ok(()));
        assert_eq!(
            assert_not_none("y", &y),
            Err(BuilderError::MissingRequiredField("y".to_string()))
        );
    }

    struct TestStruct {
        #[allow(dead_code)]
        some_field: Option<String>,
    }

    #[test]
    fn stringify_from_struct() {
        let _str = TestStruct {
            some_field: Some("some value".to_string()),
        };
        assert_eq!(stringify!(_str.some_field), "_str.some_field");
    }
}
