use anyhow::Result;
use std::path::PathBuf;

use crate::config::Target;

pub struct DiskMonitor {
    pub path: PathBuf,
}

impl DiskMonitor {
    pub fn usage(&self) -> Result<Usage> {
        let total = fs4::total_space(&self.path)?;
        Ok(Usage {
            total,
            used: total - fs4::available_space(&self.path)?,
        })
    }

    pub fn under_target(&self, target: &Target) -> Result<bool> {
        let u = self.usage()?;
        Ok(match *target {
            Target::UsedPercent(p) => (u.used as f64 / u.total as f64) * 100.0 <= p,
            Target::UsedBytes(b) => u.used <= b,
        })
    }
}

pub struct Usage {
    pub total: u64,
    pub used: u64,
}
