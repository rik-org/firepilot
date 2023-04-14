
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(rename(serialize = "boot-source"))]
    pub boot_source: BootSource,
    pub drives: Vec<Drive>,
    #[serde(rename(serialize = "network-interfaces"))]
    pub network_interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BootSource {
    /// Host level path to the kernel image used to boot the guest
    pub kernel_image_path: PathBuf,
    /// Kernel boot arguments
    pub boot_args: Option<String>,
    /// Host level path to the initrd image used to boot the guest
    pub initrd_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Drive {
    /// The identifier of the drive
    pub drive_id: String,
    /// Host level path for the guest drive
    pub path_on_host: PathBuf,
    pub is_root_device: bool,
    pub is_read_only: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkInterface {
    pub iface_id: String,
    pub guest_mac: Option<String>,
    /// Host level path for the guest network interface
    pub host_dev_name: String,
}

#[derive(Debug, Clone)]
pub struct MicroVM {
    pub id: String,
    pub config: Config,
}

impl MicroVM {
    fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl From<Config> for MicroVM {
    fn from(value: Config) -> Self {
        Self {
            id: Self::id(),
            config: value,
        }
    }
}
