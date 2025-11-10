use crate::builder::{Builder, BuilderError};
use firepilot_models::models::Vsock;

use super::assert_not_none;

#[derive(Debug)]
pub struct VsockBuilder {
    pub guest_cid: Option<i32>,
    pub uds_path: Option<String>,
}

impl Default for VsockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VsockBuilder {
    pub fn new() -> VsockBuilder {
        VsockBuilder {
            guest_cid: None,
            uds_path: None,
        }
    }

    pub fn with_guest_cid(mut self, guest_cid: i32) -> VsockBuilder {
        self.guest_cid = Some(guest_cid);
        self
    }

    pub fn with_uds_path(mut self, uds_path: String) -> VsockBuilder {
        self.uds_path = Some(uds_path);
        self
    }
}

impl Builder<Vsock> for VsockBuilder {
    fn try_build(self) -> Result<Vsock, BuilderError> {
        assert_not_none(stringify!(self.guest_cid), &self.guest_cid)?;
        assert_not_none(stringify!(self.uds_path), &self.uds_path)?;
        Ok(Vsock {
            guest_cid: self.guest_cid.unwrap(),
            uds_path: self.uds_path.unwrap(),
            vsock_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::vsock::VsockBuilder;
    use crate::builder::Builder;

    #[test]
    fn full_kernel() {
        VsockBuilder::new()
            .with_guest_cid(3)
            .with_uds_path("/tmp/fc.sock".to_string())
            .try_build()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn partial_kernel() {
        VsockBuilder::new().with_guest_cid(3).try_build().unwrap();
    }
}
