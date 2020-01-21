#[cfg(target_os = "windows")]
use winres;
#[cfg(target_os = "windows")]
use winapi;

#[cfg(target_os = "windows")]
fn main() {
    use std::io::Write;
    // only build the resource for release builds
    // as calling rc.exe might be slow
    if std::env::var("PROFILE").unwrap() == "release" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("DisZero.ico")
            .set_language(
                winapi::um::winnt::MAKELANGID(
                    winapi::um::winnt::LANG_KOREAN,
                    winapi::um::winnt::SUBLANG_KOREAN
                )
            )
            .set_manifest_file("manifest.xml");
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}
