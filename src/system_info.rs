use nvml_wrapper::Nvml;
use once_cell::sync::Lazy;
use windows::Win32::{
    Foundation::FILETIME,
    System::{
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
};

use crate::errors::Result;

static NVML: Lazy<Option<Nvml>> = Lazy::new(|| match Nvml::init() {
    Err(err) => {
        println!("{}", err);
        None
    }
    Ok(nvml) => Some(nvml),
});

pub struct SystemInfo {
    last_total_time: u64,
    last_total_exec_time: u64,
}

impl SystemInfo {
    pub fn new() -> Result<Self> {
        Ok(Self {
            last_total_time: 0,
            last_total_exec_time: 0,
        })
    }

    pub fn get_cpu_usage(&mut self) -> Result<f64> {
        let mut idle_time = unsafe { std::mem::zeroed::<FILETIME>() };
        let mut kernel_time = unsafe { std::mem::zeroed::<FILETIME>() };
        let mut user_time = unsafe { std::mem::zeroed::<FILETIME>() };

        unsafe {
            GetSystemTimes(
                Some(&mut idle_time),
                Some(&mut kernel_time),
                Some(&mut user_time),
            )
        }?;

        let idle_exec_time = filetime_as_u64(idle_time);
        let kernel_exec_time = filetime_as_u64(kernel_time);
        let user_exec_time = filetime_as_u64(user_time);

        // Kernel time includes idle time
        let total_time = kernel_exec_time + user_exec_time;
        let total_exec_time = total_time - idle_exec_time;

        let total_time_diff = total_time - self.last_total_time;
        let total_exec_time_diff = total_exec_time - self.last_total_exec_time;

        self.last_total_time = total_time;
        self.last_total_exec_time = total_exec_time;

        // TODO: This is an undercount compared to Task Manager, why?
        let exec_time_ratio = (total_exec_time_diff as f64) / (total_time_diff as f64);

        Ok(exec_time_ratio)
    }

    pub fn get_memory_usage(&self) -> Result<f64> {
        let mut mem_info = unsafe { std::mem::zeroed::<MEMORYSTATUSEX>() };
        mem_info.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        unsafe { GlobalMemoryStatusEx(&mut mem_info) }?;

        let physical_mem_used = mem_info.ullTotalPhys - mem_info.ullAvailPhys;
        let mem_ratio = (physical_mem_used as f64) / (mem_info.ullTotalPhys as f64);

        Ok(mem_ratio)
    }

    pub fn get_gpu_usage(&self) -> Result<f64> {
        if let Some(nvml) = NVML.as_ref() {
            // TODO: Support other GPUs somehow
            let gpu = nvml.device_by_index(0)?;
            let usage = gpu.utilization_rates()?;
            let usage_ratio = usage.gpu as f64 / 100.0;
            return Ok(usage_ratio);
        }

        Ok(0.0)
    }
}

fn filetime_as_u64(filetime: FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | (filetime.dwLowDateTime as u64)
}
