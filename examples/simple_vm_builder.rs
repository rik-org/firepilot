use std::path::PathBuf;

use firepilot::builder::{BuilderError, Builder, Configuration, kernel::KernelBuilder, drive::DriveBuilder, executor::FirecrackerExecutorBuilder};

fn main() -> Result<(), BuilderError> {
    let kernel = KernelBuilder::new()
        .with_kernel_image_path("path/to/kernel".to_string())
        .with_initrd_path("path/to/initrd".to_string())
        .with_boot_args("console=ttyS0 reboot=k panic=1 pci=off".to_string())
        .try_build()?;
    let drive = DriveBuilder::new()
        .with_drive_id("rootfs".to_string())
        .with_path_on_host(PathBuf::from("/path/to/rootfs"))
        .as_read_only()
        .as_root_device()
        .try_build()?;
    let executor = FirecrackerExecutorBuilder::new()
        .with_chroot("/".to_string())
        .with_exec_binary(PathBuf::from("/usr/bin/firecracker"))
        .try_build()?;
    Configuration::new()
        .with_kernel(kernel)
        .with_executor(executor)
        .with_drive(drive)
        .try_build()?;
    Ok(())
}