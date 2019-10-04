use crate::mem_lib;
use failure::Error;

pub const STARCRAFT_VERSION: &str = "1.23.1.6623";

enum Offset {
    Buffer = 0xBFD6E8,
    DropTimer = 0xC324F4,
    Version = 0x9C6988,
}

pub struct StarCraft {
    process: mem_lib::GameProcess,
    module: mem_lib::Module,
    pub state: State,
    pub event: Event,
}

#[derive(PartialEq)]
pub enum State {
    WaitingStarCraft,
    WaitingSCBankMap,
    RequestFilename,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Event {
    NotHappened,
    Found,
    Mismatched,
    Failed,
}

impl Default for StarCraft {
    fn default() -> StarCraft {
        let proc = mem_lib::GameProcess::current_process();
        let s = std::env::current_exe().unwrap();
        let s = s.file_name().unwrap();
        let s = s.to_str().unwrap();
        let module = proc.get_module(s).unwrap();
        StarCraft {
            process: proc,
            module: module,
            state: State::WaitingStarCraft,
            event: Event::NotHappened,
        }
    }
}

impl StarCraft {
    pub fn next(&mut self) {
        let state = &self.state;
        let event = &self.event;
        self.state = match (state, event) {
            (State::WaitingStarCraft, Event::Found) => {
                self.event = Event::NotHappened;
                State::WaitingSCBankMap
            }
            (State::WaitingSCBankMap, Event::Found) => {
                self.event = Event::NotHappened;
                State::RequestFilename
            }
            (State::WaitingSCBankMap, Event::Failed) => State::WaitingStarCraft,
            (_s, _e) => {
                return;
            }
        };
    }
    pub fn run(&mut self) {
        match self.state {
            State::WaitingStarCraft => {
                let (process, module) = match get_proc_and_module() {
                    Ok((p, m)) => (p, m),
                    Err(_) => {
                        self.event = Event::NotHappened;
                        return;
                    }
                };
                let version = module
                    .read::<[u8; 11]>(Offset::Version as u32, &process)
                    .unwrap_or([0; 11]);
                self.event = if STARCRAFT_VERSION.as_bytes() == version {
                    self.process = process;
                    self.module = module;
                    Event::Found
                } else {
                    Event::Mismatched
                };
            }
            State::WaitingSCBankMap => {
                self.event = match self
                    .module
                    .read::<u32>(Offset::Buffer as u32 + 212, &self.process)
                    {
                        Ok(value) => {
                            if value == 0x5537F23B {
                                Event::Found
                            } else {
                                Event::Mismatched
                            }
                        }
                        Err(_) => Event::Failed,
                    }
            }
            _ => (),
        }
        if self.state != State::WaitingStarCraft {
            match self.module.read::<u32>(Offset::DropTimer as u32, &self.process) {
                Ok(timer) => {
                    if timer >= 1 {
                        self
                            .module
                            .write::<u32>(&self.process, Offset::DropTimer as u32, 0)
                            .unwrap();
                        println!("드랍 타이머가 0으로 설정되었습니다.");
                    }
                }
                Err(_) => {
                    self.state = State::WaitingStarCraft;
                }
            };
        }
    }
}

fn get_proc_and_module() -> Result<(mem_lib::GameProcess, mem_lib::Module), Error> {
    let process = mem_lib::get_proc_by_name("StarCraft.exe")?;
    let module = process.get_module("StarCraft.exe")?;
    Ok((process, module))
}
