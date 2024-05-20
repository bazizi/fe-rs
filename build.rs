#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    vergen::EmitBuilder::builder().all_build().all_git().emit()?;
    extern crate winres;
    let mut res = winres::WindowsResource::new();
    res.set_icon("res/app.ico"); // Replace this with the filename of your .ico file.
    res.compile().unwrap();
    Ok(())
}

#[cfg(target_os = "linux")]
fn main() {
    // no op
}
