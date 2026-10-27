//! What the machine has, for recommending model sizes.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub ram_gb: f64,
    pub cpu: String,
    pub os: String,
    pub arch: &'static str,
    pub cores: usize,
    pub recommended_backbone: &'static str,
    pub recommended_llm: &'static str,
}

pub fn ram_gb() -> f64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.total_memory() as f64 / 1024f64.powi(3)
}

pub fn info() -> SystemInfo {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.refresh_cpu_all();
    let ram = system.total_memory() as f64 / 1024f64.powi(3);
    SystemInfo {
        ram_gb: (ram * 10.0).round() / 10.0,
        cpu: system.cpus().first().map(|cpu| cpu.brand().trim().to_string()).unwrap_or_default(),
        os: sysinfo::System::long_os_version().unwrap_or_default(),
        arch: std::env::consts::ARCH,
        cores: system.cpus().len(),
        recommended_backbone: crate::catalog::recommended_backbone(ram),
        recommended_llm: crate::catalog::recommended_llm(ram),
    }
}
