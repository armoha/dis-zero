use std::{ffi::OsString, mem, os::windows::ffi::OsStringExt};
// source: https://www.unknowncheats.me/forum/general-programming-and-reversing/330583-pure-rust-injectable-dll.html
use std::result::Result as StdResult;

use failure::{Error as FError, Fail};
use getset::Getters;
use winapi::um::{handleapi, memoryapi, processthreadsapi, tlhelp32, winnt};

#[derive(Getters)]
#[get = "pub"]
pub struct GameProcess {
    handle: winnt::HANDLE,
    pid: u32,
}

#[derive(Getters, Default)]
#[get = "pub"]
pub struct Module {
    base: u32,
}

pub type Result<T> = StdResult<T, FError>;

#[derive(Debug, Fail)]
pub enum ProcessErrorKind {
    #[fail(display = "Couldn't read memory at {:#X}", _0)]
    MemoryRead(u32),

    #[fail(display = "CreateToolhelp32Snapshot returned INVALID_HANDLE_VALUE")]
    InvalidHandleValue,

    #[fail(display = "Unknown process: {}", _0)]
    UnknownProcess(String),

    #[fail(display = "Unknown module: {}", _0)]
    UnknownModule(String),

    #[fail(display = "InvalidBytesWritten: {}", _0)]
    InvalidBytesWritten(u32),
}

impl GameProcess {
    pub fn current_process() -> Self {
        Self::new(unsafe { processthreadsapi::GetCurrentProcess() })
    }

    pub fn new(handle: winnt::HANDLE) -> Self {
        let pid = unsafe { processthreadsapi::GetProcessId(handle) };
        GameProcess { handle, pid }
    }

    pub fn get_module(&self, module_name: &str) -> Result<Module> {
        let module =
            unsafe { tlhelp32::CreateToolhelp32Snapshot(tlhelp32::TH32CS_SNAPMODULE, self.pid) };
        if module == handleapi::INVALID_HANDLE_VALUE {
            return Err(ProcessErrorKind::InvalidHandleValue.into());
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

                if cfg!(debug_assertions) {
                    println!(
                        "Base address of {}: 0x{:X} @ size of 0x{:X}",
                        module_name, entry.modBaseAddr as u32, entry.modBaseSize
                    );
                }

                return Ok(Module {
                    base: entry.modBaseAddr as _,
                });
            }
        }

        Err(ProcessErrorKind::UnknownModule(module_name.into()).into())
    }
}

pub fn get_proc_by_name(name: &str) -> Result<GameProcess> {
    let mut process: tlhelp32::PROCESSENTRY32W = unsafe { mem::MaybeUninit::uninit().assume_init() };
    process.dwSize = mem::size_of::<tlhelp32::PROCESSENTRY32W>() as u32;

    //Make a Snapshot of all the current process.
    let snapshot = unsafe { tlhelp32::CreateToolhelp32Snapshot(tlhelp32::TH32CS_SNAPPROCESS, 0) };

    //Get the first process and store it in process variable.
    if unsafe { tlhelp32::Process32FirstW(snapshot, &mut process) } != 0 {
        //Take the next process if possible.
        while unsafe { tlhelp32::Process32NextW(snapshot, &mut process) } != 0 {
            let process_name = OsString::from_wide(&process.szExeFile);

            match process_name.into_string() {
                Ok(s) => {
                    if s.contains(name) {
                        return Ok(GameProcess {
                            handle: unsafe {
                                processthreadsapi::OpenProcess(
                                    winnt::PROCESS_VM_READ
                                        | winnt::PROCESS_VM_OPERATION
                                        | winnt::PROCESS_VM_WRITE,
                                    0,
                                    process.th32ProcessID,
                                )
                            },
                            pid: process.th32ProcessID,
                        });
                    }
                }
                Err(_) => {
                    println!(
                        "Error converting process name for PID {}",
                        process.th32ProcessID
                    );
                }
            }
        }
    }
    Err(ProcessErrorKind::UnknownProcess(name.into()).into())
}

impl Module {
    pub fn read<T>(&self, offset: u32, parent: &GameProcess) -> Result<T> {
        let mut read = unsafe { mem::MaybeUninit::uninit().assume_init() };
        let mut amount_read: libc::size_t = 0;

        if unsafe {
            memoryapi::ReadProcessMemory(
                *(&*parent).handle(),
                (self.base + offset) as *const _,
                &mut read as *mut _ as *mut _,
                mem::size_of::<T>() as _,
                &mut amount_read as *mut _,
            )
        } != (true as i32)
            || amount_read == 0
        {
            mem::forget(read);
            return Err(ProcessErrorKind::MemoryRead(self.base + offset).into());
        }

        Ok(read)
    }

    pub fn write<T>(&mut self, parent: &GameProcess, offset: u32, mut value: T) -> Result<()> {
        if unsafe {
            memoryapi::WriteProcessMemory(
                *(&*parent).handle(),
                (self.base + offset) as *mut _,
                &mut value as *mut _ as *mut _,
                mem::size_of_val(&value),
                std::ptr::null_mut(),
            ) as usize
        } == 0
        {
            return Err(ProcessErrorKind::InvalidBytesWritten(self.base + offset).into());
        }

        Ok(())
    }
}
