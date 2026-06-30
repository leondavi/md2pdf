//! Build script.
//!
//! On Windows, embed the application icon into the produced executables so the
//! GUI app and CLI show the md2pdf "2" icon in Explorer and the taskbar.

fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/logo.ico");
        // Don't fail the whole build if the resource compiler is unavailable.
        let _ = res.compile();
    }
    println!("cargo:rerun-if-changed=assets/logo.ico");
}
