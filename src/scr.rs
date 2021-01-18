use crate::mem_lib::{Error, ProcessInfo};
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
            version: "1.23.7.9198".to_string(),
            version_offset32: 0xB4D210,
            version_offset64: 0xDE6CF8,
            droptimer_offset32: 0xDD0F3C,
            droptimer_offset64: 0x10B2C5C,
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
    #[allow(dead_code)]
    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashMap;

        let url = "https://raw.githubusercontent.com/armoha/dis-zero/master/scdata.json";
        let resp = reqwest::blocking::get(url)?.json::<HashMap<String, String>>()?;

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
    pub fn get_sc_pinfo() -> Result<ProcessInfo, Error> {
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
                self.process = match SCInfo::get_sc_pinfo() {
                    Ok(p) => p,
                    Err(_) => {
                        self.event = Event::NotHappened;
                        return;
                    }
                };
                let version_offset = self.get_version_offset();
                use std::str;
                if let Ok(my_version) = self
                    .process
                    .read_address(version_offset, self.scdata.version.len())
                {
                    if let Ok(t) = str::from_utf8(&my_version) {
                        println!("버전: {}", t);
                        if self.scdata.version == t {
                            self.event = Event::Found;
                            return;
                        } else {
                            self.event = Event::Mismatched;
                        };
                    }
                }
                use std::io::stdin;
                let mut latest_version = String::new();
                loop {
                    println!("버전 불일치! 현재 스타크래프트 버전을 입력해주세요:");
                    if let Ok(_) = stdin().read_line(&mut latest_version) {
                        if let Some('\n') = latest_version.chars().next_back() {
                            latest_version.pop();
                        }
                        if let Some('\r') = latest_version.chars().next_back() {
                            latest_version.pop();
                        }
                        break;
                    }
                }
                println!("버전 {}의 새 오프셋을 조사합니다.", latest_version);
                let mut latest_offset: usize = 0x800000;
                loop {
                    if let Ok(maybe_version) = self
                        .process
                        .read_address(latest_offset, latest_version.len())
                    {
                        if let Ok(t) = str::from_utf8(&maybe_version) {
                            if t == latest_version {
                                break;
                            }
                        }
                    }
                    latest_offset += 1;
                }
                let sc_bit = if self.process.base_addr < 0x7FFFFFFF {
                    32
                } else {
                    64
                };
                println!(
                    "{}비트 스타크래프트 {}의 버전 오프셋을 발견했습니다!",
                    sc_bit, latest_version
                );
                println!("https://github.com/armoha/dis-zero/blob/master/scdata.json 에 보고하여 업데이트를 도와주세요.");
                println!("버전 오프셋: 0x{:X}", latest_offset);
                self.event = Event::Found;
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
                #[cfg(debug_assertions)]
                {
                    use std::io::stdin;
                    println!("새 드랍 타이머 오프셋을 조사합니다. 조사용 맵을 실행하세요:");
                    let _ = stdin().read_line(&mut String::new());
                    let mut latest_drop_timer: usize = 0x800000;
                    loop {
                        drop_timer.set_offset(vec![self.process.base_addr + latest_drop_timer]);
                        if let Ok(d) = drop_timer.read() {
                            if d == 0xEDACEDAC {
                                println!("1st match: 0x{:X}", latest_drop_timer);
                                let _ = stdin().read_line(&mut String::new());
                                if let Ok(e) = drop_timer.read() {
                                    if e == 0xCADECADE {
                                        break;
                                    } else {
                                        println!("mismatch!");
                                        let _ = stdin().read_line(&mut String::new());
                                    }
                                }
                            }
                        }
                        latest_drop_timer += 1;
                    }
                    println!("새 드랍 타이머 오프셋: 0x{:X}", latest_drop_timer);
                }
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
