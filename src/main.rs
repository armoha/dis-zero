mod mem_lib;
mod scr;
use std::{thread, time};

fn main() {
    let sc = &mut scr::SCInfo::default();
    println!("스타크래프트 최신 정보 가져오는 중...");
    #[cfg(not(debug_assertions))]
    if let Err(e) = sc.update() {
        println!(
            "스타크래프트 최신 정보를 가져오는데 실패했습니다. 인터넷 연결을 확인하세요\n: {:?}",
            e
        );
    };
    let wait = time::Duration::from_millis(80);
    println!(
        "스타크래프트 리마스터 {}버전만 지원합니다.",
        sc.scdata.version
    );
    loop {
        sc.next();
        sc.run();
        thread::sleep(wait);
    }
}
