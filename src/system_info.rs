use windows::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};

use crate::errors::Result;

pub struct SystemInfo {
    last_total_time: u64,
    last_total_exec_time: u64,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            last_total_time: 0,
            last_total_exec_time: 0,
        }
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
}

fn filetime_as_u64(filetime: FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | (filetime.dwLowDateTime as u64)
}
