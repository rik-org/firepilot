use std::{
    env::{split_paths, var_os},
    path::PathBuf,
};

use crate::{
    builder::{Builder, BuilderError},
    executor::{Executor, FirecrackerExecutor},
};

use super::assert_not_none;

#[derive(Debug)]
pub struct FirecrackerExecutorBuilder {
    chroot: Option<String>,
    exec_binary: Option<PathBuf>,
}

impl FirecrackerExecutorBuilder {
    pub fn new() -> FirecrackerExecutorBuilder {
        FirecrackerExecutorBuilder {
            chroot: None,
            exec_binary: None,
        }
    }

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
    pub fn determine_binary_location() -> Result<PathBuf, BuilderError> {
        Self::find_binary_from_env_location()
            .or_else(Self::find_binary_from_path)
            .or_else(Self::find_binary_from_current_directory)
            .map(|p| Ok(p))
            .unwrap_or(Err(BuilderError::BinaryNotFound("Check if FIRECRACKER_LOCATION environment variable is correctly set. For more information check https://docs.rs/firepilot/ ".to_string())))
    }

    /// Create a new firecracker executor, it will try to determine the binary location, but you can
    /// provide a custom one through several options (upper take priority over lower):
    ///
    /// - `FIRECRACKER_LOCATION` environment variable: direct path to the binary
    /// - `$PATH` environment variable: search for the binary in the directories
    /// - `firecracker` binary in the current working directory
    ///
    /// If you provided a custom path to the binary and the binary doesn't exist it will return
    /// [BuilderError::BinaryNotFound].
    ///
    /// If you don't provide a directory to store `firecracker` related files, it will use the
    /// default one ("/srv").
    pub fn auto() -> Result<FirecrackerExecutorBuilder, BuilderError> {
        let binary_path = Self::determine_binary_location()?;
        let chroot = "/srv".to_string();

        let builder = Self::new()
            .with_chroot(chroot)
            .with_exec_binary(binary_path);

        Ok(builder)
    }

    pub fn with_chroot(mut self, chroot: String) -> FirecrackerExecutorBuilder {
        self.chroot = Some(chroot);
        self
    }

    pub fn with_exec_binary(mut self, exec_binary: PathBuf) -> FirecrackerExecutorBuilder {
        self.exec_binary = Some(exec_binary);
        self
    }
}

impl Builder<Executor> for FirecrackerExecutorBuilder {
    fn try_build(self) -> Result<Executor, BuilderError> {
        assert_not_none(stringify!(self.chroot), &self.chroot)?;
        assert_not_none(stringify!(self.exec_binary), &self.exec_binary)?;
        let executor = FirecrackerExecutor {
            chroot: self.chroot.unwrap(),
            exec_binary: self.exec_binary.unwrap(),
        };
        Ok(Executor::new_with_firecracker(executor))
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use std::env::var_os;
    use std::fs::File;

    use tempfile::tempdir;

    use crate::builder::executor::FirecrackerExecutorBuilder;
    #[test]
    fn test_firecracker_executor_builder() {
        use super::FirecrackerExecutorBuilder;
        use crate::builder::Builder;
        use std::path::PathBuf;

        FirecrackerExecutorBuilder::new()
            .with_chroot("/".to_string())
            .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
            .try_build()
            .unwrap();
    }

    #[test]
    fn test_firecracker_executor_required_fields() {
        use super::FirecrackerExecutorBuilder;
        use crate::builder::Builder;
        use std::path::PathBuf;

        let result = FirecrackerExecutorBuilder::new()
            .with_chroot("/".to_string())
            .try_build();
        assert!(result.is_err());

        let result = FirecrackerExecutorBuilder::new()
            .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
            .try_build();
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_can_determine_binary_location_from_env() {
        let dir = tempdir().expect("failed to create temporary directory");
        let file_path = dir.path().join("firecracker");
        let _file = File::create(file_path.clone()).expect("failed to create temporary file");
        std::env::set_var("FIRECRACKER_LOCATION", file_path);
        let result = FirecrackerExecutorBuilder::determine_binary_location();
        assert!(result.is_ok());
        std::env::remove_var("FIRECRACKER_LOCATION");
    }

    #[test]
    #[serial]
    fn test_cant_determine_binary_location_from_env() {
        std::env::set_var("FIRECRACKER_LOCATION", "/tmp/invalid_path/firecracker");
        let result = FirecrackerExecutorBuilder::find_binary_from_env_location();
        assert!(result.is_none());
        std::env::remove_var("FIRECRACKER_LOCATION");
    }

    #[test]
    fn test_can_determine_binary_location_from_path() {
        let dir = tempdir().expect("failed to create temporary directory");
        let file_path = dir.path().join("firecracker");
        let _file = File::create(file_path.clone()).expect("failed to create temporary file");

        std::env::set_var("PATH", file_path.parent().unwrap());
        println!("{:?}", var_os("PATH"));
        let result = FirecrackerExecutorBuilder::determine_binary_location();
        assert!(result.is_ok())
    }

    #[test]
    fn test_cant_determine_binary_location_from_path() {
        std::env::set_var("PATH", "/tmp/invalid_path");
        let result = FirecrackerExecutorBuilder::determine_binary_location();
        assert!(result.is_err())
    }
}
