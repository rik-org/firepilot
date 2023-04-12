use crate::{builder::{Builder, BuilderError}, models::BootSource};

use super::assert_not_none;

pub struct KernelBuilder {
    pub boot_args: Option<String>,
    pub initrd_path: Option<String>,
    pub kernel_image_path: Option<String>,
}

impl KernelBuilder {
    pub fn new() -> KernelBuilder {
        KernelBuilder {
            boot_args: None,
            initrd_path: None,
            kernel_image_path: None,
        }
    }

    pub fn with_boot_args(mut self, boot_args: String) -> KernelBuilder {
        self.boot_args = Some(boot_args);
        self
    }

    pub fn with_initrd_path(mut self, initrd_path: String) -> KernelBuilder {
        self.initrd_path = Some(initrd_path);
        self
    }

    pub fn with_kernel_image_path(mut self, kernel_image_path: String) -> KernelBuilder {
        self.kernel_image_path = Some(kernel_image_path);
        self
    }
}

impl Builder<BootSource> for KernelBuilder {
    fn try_build(self) -> Result<BootSource, BuilderError> {
        assert_not_none(stringify!(self.kernel_image_path), &self.kernel_image_path)?;
        Ok(BootSource {
            kernel_image_path: self.kernel_image_path.unwrap(),
            initrd_path: self.initrd_path,
            boot_args: self.boot_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::kernel::KernelBuilder;
    use crate::builder::Builder;

    #[test]
    fn full_kernel() {
        KernelBuilder::new()
            .with_kernel_image_path("path/to/kernel".to_string())
            .with_initrd_path("path/to/initrd".to_string())
            .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off".to_string())
            .try_build()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn partial_kernel() {
        KernelBuilder::new()
            .with_initrd_path("path/to/initrd".to_string())
            .try_build()
            .unwrap();
    }
}