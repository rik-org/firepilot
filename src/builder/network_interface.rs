use crate::models::{NetworkInterface, RateLimiter};

use super::{assert_not_none, Builder, BuilderError};

pub struct NetworkInterfaceBuilder {
    guest_mac: Option<String>,
    host_dev_name: Option<String>,
    iface_id: Option<String>,
    rx_rate_limiter: Option<Box<RateLimiter>>,
    tx_rate_limiter: Option<Box<RateLimiter>>,
}

impl NetworkInterfaceBuilder {
    pub fn new() -> NetworkInterfaceBuilder {
        NetworkInterfaceBuilder {
            guest_mac: None,
            host_dev_name: None,
            iface_id: None,
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        }
    }

    pub fn with_guest_mac(mut self, guest_mac: String) -> NetworkInterfaceBuilder {
        self.guest_mac = Some(guest_mac);
        self
    }

    pub fn with_host_dev_name(mut self, host_dev_name: String) -> NetworkInterfaceBuilder {
        self.host_dev_name = Some(host_dev_name);
        self
    }

    pub fn with_iface_id(mut self, iface_id: String) -> NetworkInterfaceBuilder {
        self.iface_id = Some(iface_id);
        self
    }

    pub fn with_rx_rate_limiter(
        mut self,
        rx_rate_limiter: Box<RateLimiter>,
    ) -> NetworkInterfaceBuilder {
        self.rx_rate_limiter = Some(rx_rate_limiter);
        self
    }

    pub fn with_tx_rate_limiter(
        mut self,
        tx_rate_limiter: Box<RateLimiter>,
    ) -> NetworkInterfaceBuilder {
        self.tx_rate_limiter = Some(tx_rate_limiter);
        self
    }
}

impl Builder<NetworkInterface> for NetworkInterfaceBuilder {
    fn try_build(self) -> Result<NetworkInterface, BuilderError> {
        assert_not_none(stringify!(self.host_dev_name), &self.host_dev_name)?;
        assert_not_none(stringify!(self.iface_id), &self.iface_id)?;
        Ok(NetworkInterface {
            guest_mac: self.guest_mac,
            host_dev_name: self.host_dev_name.unwrap(),
            iface_id: self.iface_id.unwrap(),
            rx_rate_limiter: self.rx_rate_limiter,
            tx_rate_limiter: self.tx_rate_limiter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iface_builder() {
        let iface = NetworkInterfaceBuilder::new()
            .with_host_dev_name("eth0".to_string())
            .with_iface_id("net0".to_string())
            .try_build()
            .unwrap();
        assert_eq!(iface.host_dev_name, "eth0");
        assert_eq!(iface.iface_id, "net0");
    }

    #[test]
    #[should_panic]
    fn test_iface_incomplete() {
        let _ = NetworkInterfaceBuilder::new().try_build().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iface_no_host_dev_name() {
        let _ = NetworkInterfaceBuilder::new()
            .with_iface_id("net0".to_string())
            .try_build()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn test_iface_no_iface_id() {
        let _ = NetworkInterfaceBuilder::new()
            .with_host_dev_name("eth0".to_string())
            .try_build()
            .unwrap();
    }
}
