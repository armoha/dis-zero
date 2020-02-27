use proclist;
use std::{ffi::OsString, io, mem, os::windows::ffi::OsStringExt};
use process_memory::{Pid, TryIntoProcessHandle, copy_address};
use winapi::um::{tlhelp32, handleapi};

pub struct ProcessInfo {
    pub pid: Pid,
    pub base_addr: usize,
}

pub enum Error {
    NoProcessFound,
    InvalidHandleValue,
    UnknownModule,
}

impl Default for ProcessInfo {
    fn default() -> ProcessInfo {
        use std::process;

        ProcessInfo {
            pid: process::id(),
            base_addr: 0,
        }
    }
}

impl ProcessInfo {
    pub fn get_pinfo_by_name(name: &str) -> Result<ProcessInfo, Error> {
        for process_info in proclist::iterate_processes_info().filter_map(|r| r.ok()) {
            if process_info.name == name {
                let base_addr = ProcessInfo::get_modbaseaddr(process_info.pid, name)?;
                return Ok(ProcessInfo {
                    pid: process_info.pid,
                    base_addr: base_addr,
                });
            }
        }
        Err(Error::NoProcessFound)
    }

    fn get_modbaseaddr(pid: Pid, module_name: &str) -> Result<usize, Error> {
        let module =
            unsafe { tlhelp32::CreateToolhelp32Snapshot(tlhelp32::TH32CS_SNAPMODULE, pid) };
        if module == handleapi::INVALID_HANDLE_VALUE {
            return Err(Error::InvalidHandleValue);
        }

        let mut entry: tlhelp32::MODULEENTRY32W = unsafe { mem::zeroed() };
        entry.dwSize = mem::size_of::<tlhelp32::MODULEENTRY32W>() as _;

        while unsafe { tlhelp32::Module32NextW(module, &mut entry) } != 0 {
            let name = OsString::from_wide(&entry.szModule[..]).into_string();
            let name = match name {
                Err(e) => {
                    eprintln!("Couldn't convert into String: {:?}", e);
                    continue;
                }
                Ok(s) => s,
            };

            if name.contains(module_name) {
                unsafe { handleapi::CloseHandle(module) };

                /*println!(
                    "Base address of {}: 0x{:X} @ size of 0x{:X}",
                    module_name, entry.modBaseAddr as usize, entry.modBaseSize
                );*/

                return Ok(entry.modBaseAddr as _);
            }
        }

        Err(Error::UnknownModule)
    }

    pub fn read_address(&self, address: usize, size: usize) -> io::Result<Vec<u8>> {
        let handle = self.pid.try_into_process_handle()?;
        let bytes = copy_address(self.base_addr + address, size, &handle)?;
        Ok(bytes)
    }
}
