use failure::Error;

#[cfg(windows)]
fn add_icon() -> Result<(), Error> {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/SCBank.ico");
    res.compile()?;
    Ok(())
}

#[cfg(windows)]
fn main() -> Result<(), Error> {
    add_icon()?;
    Ok(())
}
