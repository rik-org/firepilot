use firepilot::microvm::{BootSource, Config, Drive, MicroVM};
use firepilot::{Firecracker, FirecrackerOptions};
use std::path::PathBuf;
use std::thread;

const TEST_FIXTURES_DIR_PATH: &str = "./fixtures/";
const TEST_VMLINUX_BIN_PATH: &str = "./fixtures/vmlinux.bin";
const TEST_ROOTFS_PATH: &str = "./fixtures/rootfs.ext4";

#[test]
fn test_it_run_vm_from_config() {
    // show pwd
    println!("pwd: {}", std::env::current_dir().unwrap().display());
    // list files in TEST_FIXTURES_DIR_PATH
    for entry in std::fs::read_dir(TEST_FIXTURES_DIR_PATH).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        println!("file: {}", path.display());
    }
    let firecracker = Firecracker::new(FirecrackerOptions::default()).unwrap();
    let vm = MicroVM::from(Config {
        boot_source: BootSource {
            kernel_image_path: PathBuf::from(TEST_VMLINUX_BIN_PATH),
            boot_args: None,
            initrd_path: None,
        },
        drives: vec![Drive {
            drive_id: "rootfs".to_string(),
            path_on_host: PathBuf::from(TEST_ROOTFS_PATH),
            is_read_only: false,
            is_root_device: true,
        }],
        network_interfaces: vec![/* NetworkInterface {
                iface_id: TEST_IFACE_ID.to_string(),
                guest_mac: Some(TEST_GUEST_MAC.to_string()),
                host_dev_name: TEST_HOST_DEV_NAME.to_string(),
            } */],
    });
    thread::spawn(move || {
        firecracker.start(&vm).unwrap();
    });
}
