use std::path::PathBuf;

use crate::{
    builder::{assert_not_none, Builder, BuilderError},
    models::Drive,
};

pub struct DriveBuilder {
    pub drive_id: Option<String>,
    pub path_on_host: Option<PathBuf>,
    pub is_root_device: bool,
    pub is_read_only: bool,
}

impl DriveBuilder {
    pub fn new() -> DriveBuilder {
        DriveBuilder {
            drive_id: None,
            path_on_host: None,
            is_root_device: false,
            is_read_only: false,
        }
    }

    pub fn with_drive_id(mut self, drive_id: String) -> DriveBuilder {
        self.drive_id = Some(drive_id);
        self
    }

    pub fn with_path_on_host(mut self, path_on_host: PathBuf) -> DriveBuilder {
        self.path_on_host = Some(path_on_host);
        self
    }

    pub fn as_root_device(mut self) -> DriveBuilder {
        self.is_root_device = true;
        self
    }

    pub fn as_read_only(mut self) -> DriveBuilder {
        self.is_read_only = true;
        self
    }
}

impl Builder<Drive> for DriveBuilder {
    fn try_build(self) -> Result<Drive, BuilderError> {
        assert_not_none(stringify!(self.drive_id), &self.drive_id)?;
        assert_not_none(stringify!(self.path_on_host), &self.path_on_host)?;
        Ok(Drive {
            drive_id: self.drive_id.unwrap(),
            // FIXME: This is a hack to convert PathBuf to String
            path_on_host: self
                .path_on_host
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            is_root_device: self.is_root_device,
            is_read_only: self.is_read_only,
            cache_type: None,
            partuuid: None,
            rate_limiter: None,
            io_engine: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::{Builder, BuilderError};

    #[test]
    fn drive_full() {
        let drive = crate::builder::drive::DriveBuilder::new()
            .with_drive_id("rootfs".to_string())
            .with_path_on_host("/path/to/rootfs".into())
            .as_root_device()
            .as_read_only()
            .try_build();
        assert_eq!(drive.is_ok(), true);
    }

    #[test]
    fn drive_incomplete_path_host() {
        let drive = crate::builder::drive::DriveBuilder::new()
            .with_drive_id("rootfs".to_string())
            .try_build();
        assert_eq!(drive.is_err(), true);
        assert_eq!(
            drive.err().unwrap(),
            BuilderError::MissingRequiredField(stringify!(self.path_on_host).to_string())
        );
    }

    #[test]
    fn drive_incomplete_drive_id() {
        let drive = crate::builder::drive::DriveBuilder::new()
            .with_path_on_host("/path/to/rootfs".into())
            .try_build();
        assert_eq!(drive.is_err(), true);
        assert_eq!(
            drive.err().unwrap(),
            BuilderError::MissingRequiredField(stringify!(self.drive_id).to_string())
        );
    }
}
