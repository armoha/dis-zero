mod mem_lib;
mod scr;
use std::{thread, time};

fn main() {
    let sc = &mut scr::StarCraft::default();
    let wait = time::Duration::from_millis(80);
    println!("스타크래프트 리마스터 {}버전만 지원합니다.", scr::STARCRAFT_VERSION);
    loop {
        sc.next();
        sc.run();
        thread::sleep(wait);
    }
}