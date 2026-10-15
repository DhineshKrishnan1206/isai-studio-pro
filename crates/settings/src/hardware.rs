use crate::Profile;

/// What the first-launch setup learns about the computer.
#[derive(Clone, Debug, Default)]
pub struct Hardware {
    pub cpu_name: String,
    pub cores: usize,
    pub ram_gb: f32,
    /// Graphics renderer name, filled in by the app once a GPU context exists.
    pub gpu: Option<String>,
    /// Synth voices one core can render in real time, from the engine benchmark.
    pub voices: u32,
}

impl Hardware {
    /// Reads CPU and memory. The GPU name and benchmark result are added separately.
    pub fn detect() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
        let cpu_name = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_owned())
            .unwrap_or_default();
        let cores = sysinfo::System::physical_core_count().unwrap_or(sys.cpus().len()).max(1);
        Self {
            cpu_name,
            cores,
            ram_gb: sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0),
            gpu: None,
            voices: 0,
        }
    }

    /// True when graphics are drawn by the CPU, which makes every animation expensive.
    pub fn software_rendering(&self) -> bool {
        self.gpu.as_deref().is_some_and(|g| {
            let g = g.to_ascii_lowercase();
            ["llvmpipe", "softpipe", "swiftshader", "software", "gdi generic"]
                .iter()
                .any(|s| g.contains(s))
        })
    }

    /// Picks a starting profile. The user can always choose another.
    ///
    /// Voice thresholds are calibrated against an Apple M4, which scores about
    /// 7,600; older dual-core laptops are expected to land well under 1,000.
    pub fn recommend(&self) -> Profile {
        if self.software_rendering() || self.ram_gb < 6.0 || self.cores <= 2 || self.voices < 1000 {
            Profile::Lightweight
        } else if self.ram_gb >= 12.0 && self.cores >= 6 && self.voices >= 3000 {
            Profile::Smooth
        } else {
            Profile::Balanced
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(cores: usize, ram_gb: f32, voices: u32, gpu: &str) -> Hardware {
        Hardware { cpu_name: String::new(), cores, ram_gb, gpu: Some(gpu.into()), voices }
    }

    #[test]
    fn recommends_by_capability() {
        assert_eq!(hw(2, 4.0, 600, "Intel HD 4000").recommend(), Profile::Lightweight);
        assert_eq!(hw(4, 8.0, 2000, "Intel Iris Xe").recommend(), Profile::Balanced);
        assert_eq!(hw(8, 16.0, 7000, "Apple M2").recommend(), Profile::Smooth);
        // A fast CPU with software graphics still needs light visuals.
        assert_eq!(hw(8, 16.0, 7000, "llvmpipe (LLVM 15)").recommend(), Profile::Lightweight);
    }
}
