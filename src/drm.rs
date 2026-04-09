use std::os::fd::RawFd;

// DRM ioctl definitions
// See: /usr/include/drm/drm.h
const DRM_IOCTL_BASE: u8 = b'd';
const DRM_IOCTL_SET_MASTER: u8 = 0x1e;
const DRM_IOCTL_DROP_MASTER: u8 = 0x1f;

// ioctl request codes (no arguments, just the command)
fn drm_io(nr: u8) -> libc::c_ulong {
    // _IO('d', nr) = ((0) << 30) | (('d' as u32) << 8) | (nr as u32)
    ((DRM_IOCTL_BASE as libc::c_ulong) << 8) | (nr as libc::c_ulong)
}

/// Acquire DRM master status on a device fd.
/// This allows the process to perform modesetting operations.
pub fn set_master(fd: RawFd) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, drm_io(DRM_IOCTL_SET_MASTER)) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Drop DRM master status on a device fd.
/// This releases modesetting privileges.
pub fn drop_master(fd: RawFd) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, drm_io(DRM_IOCTL_DROP_MASTER)) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Check if a device path is a DRM device
pub fn is_drm_device(path: &std::path::Path) -> bool {
    path.to_string_lossy().starts_with("/dev/dri/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_drm_device() {
        assert!(is_drm_device(Path::new("/dev/dri/card0")));
        assert!(is_drm_device(Path::new("/dev/dri/renderD128")));
        assert!(!is_drm_device(Path::new("/dev/input/event0")));
        assert!(!is_drm_device(Path::new("/dev/tty1")));
    }

    #[test]
    fn test_drm_io_encoding() {
        // Verify the ioctl encoding matches expected values
        // DRM_IOCTL_SET_MASTER should be 0x641e ('d' << 8 | 0x1e)
        assert_eq!(drm_io(DRM_IOCTL_SET_MASTER), 0x641e);
        // DRM_IOCTL_DROP_MASTER should be 0x641f ('d' << 8 | 0x1f)
        assert_eq!(drm_io(DRM_IOCTL_DROP_MASTER), 0x641f);
    }
}
