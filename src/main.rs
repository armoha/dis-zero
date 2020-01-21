mod scr;
mod mem_lib;
use std::{thread, time};

fn main() {
    /*
    let sc = mem_lib::ProcessInfo::get_pinfo_by_name("StarCraft.exe");
    match sc {
        Ok(s) => {
            match s.read_address(0xC5_32E8, 11) {
                Ok(v) => {
                    use std::str;

                    println!("{}", str::from_utf8(&v).unwrap());
                }
                Err(e) => {
                    println!("{}", e);
                },
            }
        },
        Err(_) => ()
    };
    */
    let sc = &mut scr::SCInfo::default();
    println!("스타크래프트 최신 정보 가져오는 중...");
    if let Err(e) = sc.update() {
        println!("서버에서 스타크래프트 최신 정보를 가져오는데 실패했습니다. 인터넷 연결을 확인하세요\n: {:?}", e);
    };
    let wait = time::Duration::from_millis(80);
    println!("스타크래프트 리마스터 {}버전만 지원합니다.", sc.scdata.version);
    loop {
        sc.next();
        sc.run();
        thread::sleep(wait);
    }
}