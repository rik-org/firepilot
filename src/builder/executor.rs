use std::path::PathBuf;

use crate::{builder::{Builder, BuilderError}, executor::{FirecrackerExecutor, Executor}};

use super::assert_not_none;

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
    #[test]
    fn test_firecracker_executor_builder() {
        use std::path::PathBuf;
        use crate::builder::Builder;
        use super::FirecrackerExecutorBuilder;

        FirecrackerExecutorBuilder::new()
            .with_chroot("/".to_string())
            .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
            .try_build()
            .unwrap();
    }

    #[test]
    fn test_firecracker_executor_required_fields() {
        use std::path::PathBuf;
        use crate::builder::Builder;
        use super::FirecrackerExecutorBuilder;

        let result = FirecrackerExecutorBuilder::new()
            .with_chroot("/".to_string())
            .try_build();
        assert!(result.is_err());

        let result = FirecrackerExecutorBuilder::new()
            .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
            .try_build();
        assert!(result.is_err());
    }
}