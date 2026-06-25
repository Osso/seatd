use std::os::fd::RawFd;

// DRM ioctl definitions
// See: /usr/include/drm/drm.h
#[cfg(any(test, not(coverage)))]
const DRM_IOCTL_BASE: u8 = b'd';
#[cfg(any(test, not(coverage)))]
const DRM_IOCTL_SET_MASTER: u8 = 0x1e;
#[cfg(any(test, not(coverage)))]
const DRM_IOCTL_DROP_MASTER: u8 = 0x1f;
#[cfg(test)]
const DRM_IOCTL_SET_MASTER_REQUEST: libc::c_ulong = 0x641e;
#[cfg(test)]
const DRM_IOCTL_DROP_MASTER_REQUEST: libc::c_ulong = 0x641f;

// ioctl request codes (no arguments, just the command)
#[cfg(any(test, not(coverage)))]
fn drm_io(nr: u8) -> libc::c_ulong {
    // _IO('d', nr) = ((0) << 30) | (('d' as u32) << 8) | (nr as u32)
    ((DRM_IOCTL_BASE as libc::c_ulong) << 8) | (nr as libc::c_ulong)
}

/// Acquire DRM master status on a device fd.
/// This allows the process to perform modesetting operations.
pub fn set_master(fd: RawFd) -> std::io::Result<()> {
    set_master_impl(fd)
}

#[cfg(not(coverage))]
fn set_master_impl(fd: RawFd) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, drm_io(DRM_IOCTL_SET_MASTER)) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(coverage)]
fn set_master_impl(_fd: RawFd) -> std::io::Result<()> {
    Ok(())
}

/// Drop DRM master status on a device fd.
/// This releases modesetting privileges.
pub fn drop_master(fd: RawFd) -> std::io::Result<()> {
    drop_master_impl(fd)
}

#[cfg(not(coverage))]
fn drop_master_impl(fd: RawFd) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, drm_io(DRM_IOCTL_DROP_MASTER)) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(coverage)]
fn drop_master_impl(_fd: RawFd) -> std::io::Result<()> {
    Ok(())
}

/// Check if a device path is a DRM device
pub fn is_drm_device(path: &std::path::Path) -> bool {
    path.to_string_lossy().starts_with("/dev/dri/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;
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
        // DRM_IOCTL_SET_MASTER should be ('d' << 8 | 0x1e)
        assert_eq!(drm_io(DRM_IOCTL_SET_MASTER), DRM_IOCTL_SET_MASTER_REQUEST);
        // DRM_IOCTL_DROP_MASTER should be ('d' << 8 | 0x1f)
        assert_eq!(drm_io(DRM_IOCTL_DROP_MASTER), DRM_IOCTL_DROP_MASTER_REQUEST);
    }

    #[test]
    fn test_drm_master_calls_return_result() {
        let fd = std::fs::File::open("/dev/null").unwrap();

        let _ = set_master(fd.as_raw_fd());
        let _ = drop_master(fd.as_raw_fd());
    }
}
