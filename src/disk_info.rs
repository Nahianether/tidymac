use std::mem::MaybeUninit;

pub struct DiskInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
}

impl DiskInfo {
    pub fn usage_percent(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.used as f32 / self.total as f32
    }
}

pub fn get_disk_info() -> Option<DiskInfo> {
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
    let path = b"/\0";
    let ret = unsafe { libc::statvfs(path.as_ptr() as *const libc::c_char, stat.as_mut_ptr()) };
    if ret != 0 {
        return None;
    }
    let stat = unsafe { stat.assume_init() };
    let block_size = stat.f_frsize as u64;
    let total = stat.f_blocks as u64 * block_size;
    let available = stat.f_bavail as u64 * block_size;
    let used = total.saturating_sub(available);
    Some(DiskInfo {
        total,
        available,
        used,
    })
}
