use crate::mem_lib::{ProcessInfo, Error};
use process_memory;

pub struct SCData {
    pub version: String,
    version_offset32: usize,
    version_offset64: usize,
    droptimer_offset32: usize,
    droptimer_offset64: usize,
}

pub struct SCInfo {
    process: ProcessInfo,
    pub scdata: SCData,
    pub state: State,
    pub event: Event,
}

impl Default for SCData {
    fn default() -> SCData {
        SCData {
            version: "1.23.2.6926".to_string(),
            version_offset32: 0x9FEA18,
            version_offset64: 0xC532E8,
            droptimer_offset32: 0xC74044,
            droptimer_offset64: 0xF0B31C,
        }
    }
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

impl Default for SCInfo {
    fn default() -> SCInfo {
        SCInfo {
            process: ProcessInfo::default(),
            scdata: SCData::default(),
            state: State::WaitingStarCraft,
            event: Event::NotHappened,
        }
    }
}

impl SCInfo {
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashMap;

        let url = "https://raw.githubusercontent.com/armoha/dis-zero/master/scdata.json";
        let resp = reqwest::blocking::get(url)?
            .json::<HashMap<String, String>>()?;

        // println!("{:#?}", resp);

        for (k, v) in &resp {
            match &k[..] {
                "version" => {
                    self.scdata.version = v.to_string();
                }
                "versionOffset32" => {
                    if let Ok(x) = usize::from_str_radix(v, 16) {
                        self.scdata.version_offset32 = x;
                    }
                }
                "versionOffset64" => {
                    if let Ok(x) = usize::from_str_radix(v, 16) {
                        self.scdata.version_offset64 = x;
                    }
                }
                "dropTimerOffset32" => {
                    if let Ok(x) = usize::from_str_radix(v, 16) {
                        self.scdata.droptimer_offset32 = x;
                    }
                }
                "dropTimerOffset64" => {
                    if let Ok(x) = usize::from_str_radix(v, 16) {
                        self.scdata.droptimer_offset64 = x;
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }
    pub fn next(&mut self) {
        let state = &self.state;
        let event = &self.event;
        self.state = match (state, event) {
            (State::WaitingStarCraft, Event::Found) => {
                self.event = Event::NotHappened;
                println!("스타크래프트 버전 확인 완료");
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
    pub fn get_sc_pinfo() -> Result<ProcessInfo,Error> {
        ProcessInfo::get_pinfo_by_name("StarCraft.exe")
    }
    fn get_version_offset(&self) -> usize {
        if self.process.base_addr < 0x7FFFFFFF {
            self.scdata.version_offset32
        } else {
            self.scdata.version_offset64
        }
    }
    fn get_droptimer_offset(&self) -> usize {
        if self.process.base_addr < 0x7FFFFFFF {
            self.scdata.droptimer_offset32
        } else {
            self.scdata.droptimer_offset64
        }
    }
    pub fn run(&mut self) {
        match self.state {
            State::WaitingStarCraft => {
                let pinfo = match SCInfo::get_sc_pinfo() {
                    Ok(p) => p,
                    Err(_) => {
                        self.event = Event::NotHappened;
                        return;
                    }
                };
                let version_offset = self.get_version_offset();
                let my_version = pinfo
                    .read_address(version_offset, 11)
                    .unwrap_or(vec![0; 11]);
                use std::str;
                if let Ok(t) = str::from_utf8(&my_version) {
                    println!("버전: {}", t);
                }
                let sc_version = self.scdata.version.clone();
                self.event = if sc_version.into_bytes() == my_version {
                    self.process = pinfo;
                    Event::Found
                } else {
                    Event::Mismatched
                };
            }
            _ => {
                use process_memory::*;
                let process_handle = match self.process.pid.try_into_process_handle() {
                    Ok(h) => h,
                    Err(_) => {
                        self.event = Event::Failed;
                        return;
                    }
                };
                let mut drop_timer = DataMember::<u32>::new(process_handle);
                let drop_timer_offset = self.get_droptimer_offset();
                drop_timer.set_offset(vec![self.process.base_addr + drop_timer_offset]);
                self.event = match drop_timer.read() {
                    Ok(d) => {
                        if d >= 1 {
                            match drop_timer.write(&0) {
                                Ok(_) => {
                                    println!("드랍 타이머가 0으로 설정되었습니다.");
                                    Event::Mismatched
                                }
                                Err(_) => Event::Failed,
                            }
                        } else {
                            Event::Mismatched
                        }
                    }
                    Err(_) => Event::Failed,
                }
            }
        }
    }
}
