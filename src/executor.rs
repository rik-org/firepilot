use std::path::PathBuf;

pub struct Executor {
    firecracker: FirecrackerExecutor,
}

impl Executor {
    pub fn new_with_firecracker(firecracker: FirecrackerExecutor) -> Executor {
        Executor {
            firecracker,
        }
    }
}

pub struct FirecrackerExecutor {
    pub chroot: String,
    pub exec_binary: PathBuf,
}
