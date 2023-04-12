use crate::{models::BootSource, executor::Executor, models::Drive};

pub mod kernel;
pub mod executor;
pub mod drive;

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
    // TODO: Network Configuration
    // TODO: Log File
    executor: Option<Executor>,
    kernel: Option<BootSource>,
    storage: Vec<Drive>,
}

impl Configuration {
    pub fn new() -> Configuration {
        Configuration {
            kernel: None,
            executor: None,
            storage: Vec::new(),
        }
    }

    pub fn with_kernel(mut self, kernel: BootSource) -> Configuration {
        self.kernel = Some(kernel);
        self
    }

    pub fn with_executor(mut self, executor: Executor) -> Configuration {
        self.executor = Some(executor);
        self
    }

    pub fn with_drive(mut self, drive: Drive) -> Configuration {
        self.storage.push(drive);
        self
    }
}

impl Builder<crate::machine::Machine> for Configuration {
    fn try_build(self) -> Result<crate::machine::Machine, BuilderError> {
        assert_not_none("kernel", &self.kernel)?;
        assert_not_none("executor", &self.executor)?;
        Ok(crate::machine::Machine::new(self.executor.unwrap(), self.kernel.unwrap(), self.storage))
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::{BuilderError, assert_not_none};

    #[test]
    fn macro_assert_not_none() {
        let x = Some(1);
        let y: Option<String> = None;
        assert_eq!(assert_not_none("x", &x), Ok(()));
        assert_eq!(assert_not_none("y", &y), Err(BuilderError::MissingRequiredField("y".to_string())));
    }

    struct TestStruct {
        #[allow(dead_code)]
        some_field: Option<String>
    }

    #[test]
    fn stringify_from_struct() {
        let _str = TestStruct {
            some_field: Some("some value".to_string())
        };
        assert_eq!(stringify!(_str.some_field), "str.some_field");
    }
}