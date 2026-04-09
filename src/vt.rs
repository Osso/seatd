#![allow(dead_code)] // VT handling infrastructure - will be used when server integrates VT signals

use std::os::fd::{AsRawFd, OwnedFd, RawFd};

// VT ioctl definitions
// See: /usr/include/linux/vt.h and /usr/include/linux/kd.h

const VT_OPENQRY: libc::c_ulong = 0x5600;
const VT_GETSTATE: libc::c_ulong = 0x5603;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
const VT_WAITACTIVE: libc::c_ulong = 0x5607;
const VT_SETMODE: libc::c_ulong = 0x5602;
const VT_RELDISP: libc::c_ulong = 0x5605;
const KDSETMODE: libc::c_ulong = 0x4B3A;
const KDGKBMODE: libc::c_ulong = 0x4B44;
const KDSKBMODE: libc::c_ulong = 0x4B45;

// VT mode constants
const VT_AUTO: libc::c_short = 0;
const VT_PROCESS: libc::c_short = 1;

// KD mode constants
const KD_TEXT: libc::c_long = 0x00;
const KD_GRAPHICS: libc::c_long = 0x01;

// Keyboard modes
const K_OFF: libc::c_long = 0x04;

// VT release/acquire responses
const VT_ACKACQ: libc::c_int = 2;

#[repr(C)]
struct VtMode {
    mode: libc::c_short,
    waitv: libc::c_short,
    relsig: libc::c_short,
    acqsig: libc::c_short,
    frsig: libc::c_short,
}

#[repr(C)]
struct VtStat {
    v_active: libc::c_ushort,
    v_signal: libc::c_ushort,
    v_state: libc::c_ushort,
}

/// Virtual terminal controller
pub struct Vt {
    fd: OwnedFd,
    vt_num: u32,
    original_kb_mode: libc::c_long,
}

impl Vt {
    /// Open a specific VT by number
    pub fn open(vt_num: u32) -> std::io::Result<Self> {
        let path = format!("/dev/tty{}", vt_num);
        let fd = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;

        let raw_fd = fd.as_raw_fd();

        // Get current keyboard mode to restore later
        let mut kb_mode: libc::c_long = 0;
        let ret = unsafe { libc::ioctl(raw_fd, KDGKBMODE, &mut kb_mode) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            fd: fd.into(),
            vt_num,
            original_kb_mode: kb_mode,
        })
    }

    /// Open the current controlling TTY
    pub fn open_current() -> std::io::Result<Self> {
        let fd = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")?;

        let raw_fd = fd.as_raw_fd();

        // Get current VT number
        let mut stat = VtStat {
            v_active: 0,
            v_signal: 0,
            v_state: 0,
        };
        let ret = unsafe { libc::ioctl(raw_fd, VT_GETSTATE, &mut stat) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        // Get current keyboard mode
        let mut kb_mode: libc::c_long = 0;
        let ret = unsafe { libc::ioctl(raw_fd, KDGKBMODE, &mut kb_mode) };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            fd: fd.into(),
            vt_num: stat.v_active as u32,
            original_kb_mode: kb_mode,
        })
    }

    /// Get the VT number
    pub fn vt_num(&self) -> u32 {
        self.vt_num
    }

    /// Get the raw fd
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    /// Set VT to process mode - we handle VT switching via signals
    pub fn set_process_mode(&self, release_sig: i32, acquire_sig: i32) -> std::io::Result<()> {
        let mode = VtMode {
            mode: VT_PROCESS,
            waitv: 0,
            relsig: release_sig as libc::c_short,
            acqsig: acquire_sig as libc::c_short,
            frsig: 0,
        };

        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_SETMODE, &mode) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Set VT back to auto mode
    pub fn set_auto_mode(&self) -> std::io::Result<()> {
        let mode = VtMode {
            mode: VT_AUTO,
            waitv: 0,
            relsig: 0,
            acqsig: 0,
            frsig: 0,
        };

        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_SETMODE, &mode) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Set graphics mode (disable text console)
    pub fn set_graphics_mode(&self) -> std::io::Result<()> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KDSETMODE, KD_GRAPHICS) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Set text mode
    pub fn set_text_mode(&self) -> std::io::Result<()> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KDSETMODE, KD_TEXT) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Disable keyboard input (compositor handles it via evdev)
    pub fn disable_keyboard(&self) -> std::io::Result<()> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), KDSKBMODE, K_OFF) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Restore original keyboard mode
    pub fn restore_keyboard(&self) -> std::io::Result<()> {
        let ret =
            unsafe { libc::ioctl(self.fd.as_raw_fd(), KDSKBMODE, self.original_kb_mode) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Acknowledge VT release (allow switch away)
    pub fn ack_release(&self) -> std::io::Result<()> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_RELDISP, 1 as libc::c_int) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Acknowledge VT acquire (we're now active)
    pub fn ack_acquire(&self) -> std::io::Result<()> {
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_RELDISP, VT_ACKACQ) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Switch to a different VT
    pub fn switch_to(&self, vt_num: u32) -> std::io::Result<()> {
        let ret =
            unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_ACTIVATE, vt_num as libc::c_int) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Get current active VT
    pub fn get_active(&self) -> std::io::Result<u32> {
        let mut stat = VtStat {
            v_active: 0,
            v_signal: 0,
            v_state: 0,
        };
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_GETSTATE, &mut stat) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(stat.v_active as u32)
        }
    }

    /// Find a free VT
    pub fn find_free(&self) -> std::io::Result<u32> {
        let mut vt: libc::c_int = 0;
        let ret = unsafe { libc::ioctl(self.fd.as_raw_fd(), VT_OPENQRY, &mut vt) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(vt as u32)
        }
    }
}

impl Drop for Vt {
    fn drop(&mut self) {
        // Try to restore keyboard mode and VT auto mode
        let _ = self.restore_keyboard();
        let _ = self.set_auto_mode();
        let _ = self.set_text_mode();
    }
}

#[cfg(test)]
mod tests {
    // VT tests require actual TTY access, so we only test what we can
    use super::*;

    #[test]
    fn test_vt_constants() {
        // Just verify our constants are reasonable
        assert_eq!(VT_AUTO, 0);
        assert_eq!(VT_PROCESS, 1);
        assert_eq!(KD_TEXT, 0);
        assert_eq!(KD_GRAPHICS, 1);
    }
}
