# Firecracker VMM Configuration

An **working example** can be found [here](https://github.com/firecracker-microvm/firecracker/blob/main/tools/create_snapshot_artifact/complex_vm_config.json).

**More documentation** in firecracker repository: [Firecracker Getting started](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md#configuring-the-microvm-without-sending-api-requests)

```rust
{
    /// This struct represents the strongly typed equivalent of the json body
    /// from balloon related requests.
    "balloon": Option<{
        /// Target balloon size in MiB.
        "amount_mib": u32,
        /// Option to deflate the balloon in case the guest is out of memory.
        "deflate_on_oom": bool,
        /// Interval in seconds between refreshing statistics.
        "stats_polling_interval_s": u16,
    }>,
    /// Use this structure to set up the Block Device before booting the kernel.
    "drives": [
        {
            /// Unique identifier of the drive.
            "drive_id": String,
            /// Path of the drive.
            "path_on_host": String,
            /// If set to true, it makes the current device the root block device.
            /// Setting this flag to true will mount the block device in the
            /// guest under /dev/vda unless the partuuid is present.
            "is_root_device": bool,
            /// Part-UUID. Represents the unique id of the boot partition of this device. It is
            /// optional and it will be used only if the `is_root_device` field is true.
            "partuuid": Option<String>,
            /// If set to true, the drive is opened in read-only mode. Otherwise, the
            /// drive is opened as read-write.
            "is_read_only": bool,
            /// If set to true, the drive will ignore flush requests coming from
            /// the guest driver.
            "cache_type": Unsafe | Writeback,
            /// Rate Limiter for I/O operations.
            /// A public-facing, stateless structure, holding all the data we need to create a RateLimiter
            /// (live) object.
            "rate_limiter": {
                /// Data used to initialize the RateLimiter::bandwidth bucket.
                "bandwidth": Option<{
                    "size": u64,
                    "one_time_burst": u64,
                    "refill_time": u64,
                }>,
                /// Data used to initialize the RateLimiter::ops bucket.
                "ops": Option<{
                    "size": u64,
                    "one_time_burst": u64,
                    "refill_time": u64,
                }>,
            },
            /// The type of IO engine used by the device.
            "file_engine_type": Async | Sync,
        }
    ],
    /// Strongly typed data structure used to configure the boot source of the
    /// microvm.
    "boot-source": {
        /// Path to the kernel image.
        "kernel_image_path": String,
        /// Path to the initrd image.
        "initrd_path": Option<String>,
        /// The boot arguments to pass to the kernel. If this field is uninitialized, the default
        /// kernel command line is used: `reboot=k panic=1 pci=off nomodules 8250.nr_uarts=0`.
        "boot_args": Option<String>,
    },
    /// Strongly typed structure used to describe the logger.
    "logger": Option<{
        /// Named pipe where the logger will output the metrics.
        "log_path": PathBuf,
        /// The level of the logger
        "level": Error | Warning | Info | Debug,
        /// When enabled, the logger will append to the output the severity of the log entry.
        "show_level": bool,
        /// When enabled, the logger will append the origin of the log entry.
        "show_log_origin": bool,
    }>,
    /// Strongly typed structure that represents the configuration of the
    /// microvm.
    "machine-config": Option<{
        /// Number of vcpu to start
        "vcpu_count": u8,
        /// The memory size in MiB.
        "mem_size_mib": u32,
        /// Enables or disabled SMT.
        "smt": bool,
        /// A CPU template that it is used to filter the CPU features exposed to the guest.
        "cpu_template": C3 | T2 | T2S | None | T2CL | T2A,
        /// Enables or disables dirty page tracking. Enabling allows incremental snapshots.
        "track_dirty_pages": bool,
    }>,
    "metrics": Option<{
        /// Named pipe or file used as output for metrics.
        "metrics_path": PathBuf,
    }>,
    /// Keeps the MMDS configuration.
    "mmds-config": Option<{
        /// MMDS version.
        "version": V1 | V2,
        /// Network interfaces that allow forwarding packets to MMDS.
        "network-interface": [String],
        /// MMDS IPv4 configured address.
        "ipv4_address": Option<Ipv4Addr>,
    }>,
    /// This struct represents the strongly typed equivalent of the json body from net iface
    /// related requests.
    "network-interfaces": Vec<{
        /// ID of the guest network interface.
        "iface_id": String,
        /// Host name of the network interface.
        "host_dev_name": String,
        /// Guest MAC address.
        "guest_mac": Option<MacAddr>,
        /// Rate Limiter for receiving packages.
        /// A public-facing, stateless structure, holding all the data we need to create a RateLimiter
        /// (live) object.
        "rx_rate_limiter": {
            /// Data used to initialize the RateLimiter::bandwidth bucket.
            "bandwidth": Option<{
                /// See TokenBucket::size.
                "size": u64,
                /// See TokenBucket::one_time_burst.
                "one_time_burst": Option<u64>
                /// See TokenBucket::refill_time.
                "refill_time": u64,
            }>,
            /// Data used to initialize the RateLimiter::ops bucket.
            "ops": Option<{
                /// See TokenBucket::size.
                "size": u64,
                /// See TokenBucket::one_time_burst.
                "one_time_burst": Option<u64>
                /// See TokenBucket::refill_time.
                "refill_time": u64,
            }>,
        },
        /// Rate Limiter for transmitted packages.
        /// A public-facing, stateless structure, holding all the data we need to create a RateLimiter
        /// (live) object.
        "tx_rate_limiter": {
            /// Data used to initialize the RateLimiter::bandwidth bucket.
            "bandwidth": Option<{
                /// See TokenBucket::size.
                "size": u64,
                /// See TokenBucket::one_time_burst.
                "one_time_burst": Option<u64>
                /// See TokenBucket::refill_time.
                "refill_time": u64,
            }>,
            /// Data used to initialize the RateLimiter::ops bucket.
            "ops": Option<{
                /// See TokenBucket::size.
                "size": u64,
                /// See TokenBucket::one_time_burst.
                "one_time_burst": Option<u64>
                /// See TokenBucket::refill_time.
                "refill_time": u64,
            }>,
        }
    }>,
    /// This struct represents the strongly typed equivalent of the json body
    /// from vsock related requests.
    "vsock": Option<{
        /// ID of the vsock device.
        "vsock_id": Option<String>,
        /// A 32-bit Context Identifier (CID) used to identify the guest.
        "guest_cid": u32,
        /// Path to local unix socket.
        "uds_path": String,
    }>,
}
```
