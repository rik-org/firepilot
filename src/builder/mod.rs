use crate::{
    executor::Executor,
    models::Drive,
    models::{BootSource, NetworkInterface},
};

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
    MissingRequiredField(String),
}

pub trait Builder<T> {
    fn try_build(self) -> Result<T, BuilderError>;
}

pub struct Configuration {
    // TODO: Machine Configuration (cpu, mem...)
    // TODO: Log File
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
