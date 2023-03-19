# Firepilot - Pilot Firecracker binary through Rust

Firepilot is a **rust crate** to pilot Firecracker. It is a wrapper around Firecracker binary and provides a Rust SDK for interact with the Firecracker API.

There are some Firecracker features that are not yet supported. If you need one of them, please open an issue.

This crate is inspired by [firecracker-go-sdk](https://github.com/firecracker-microvm/firecracker-go-sdk) a Go SDK for Firecracker.

## Getting started

Add the following to your `Cargo.toml`:

```toml
[dependencies]
firepilot = { git = "https://github.com/rik-org/firepilot.git", branch = "main" }
```

Download the Firecracker binary : https://github.com/firecracker-microvm/firecracker/releases/latest

The following examples show how to use the crate:

### Run a VM

`rootfs.ext4` example : https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/rootfs/bionic.rootfs.ext4

`vmlinux.bin` example : https://s3.amazonaws.com/spec.ccfc.min/img/quickstart_guide/x86_64/kernels/vmlinux.bin

```rust
use firepilot::microvm::{BootSource, Config, Drive, MicroVM};
use firepilot::Firecracker;

let FIRECRACKER_PATH = PathBuf::from("/YOUR_PATH_HERE/firecracker");
let KERNEL_IMAGE_PATH = PathBuf::from("/YOUR_PATH_HERE/vmlinux.bin");
let ROOTFS_PATH = PathBuf::from("/YOUR_PATH_HERE/rootfs.ext4");

let firecracker = Firecracker::new(Some(firepilot::FirecrackerOptions {
                    command: Some(FIRECRACKER_PATH),
                    ..Default::default()
                }))
                .unwrap();

let vm = MicroVM::from(Config {
    boot_source: BootSource {
        kernel_image_path: KERNEL_IMAGE_PATH,
        boot_args: None,
        initrd_path: None,
    },
    drives: vec![Drive {
        drive_id: "rootfs".to_string(),
        path_on_host: ROOTFS_PATH,
        is_read_only: false,
        is_root_device: true,
    }],
    network_interfaces: vec![],
});

// Start the VM in a new thread because it is blocking
thread::spawn(move || {
    firecracker.start(&vm).unwrap();
});
```

Firepilot configure the microVM with a config file. See more documentation [here](./docs/firecracker-vmm-config.md).

## Developing

### Build

It requires to have `cargo 1.66` or higher and `protobuf-compiler` installed.

```
apt update -y && apt install -y protobuf-compiler
cargo build
```

## License

This project is released under the MIT license. Please see the [LICENSE](LICENSE) file for more information.
