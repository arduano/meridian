fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=../../assets/icons/meridian.ico");
    slint_build::compile("ui/app.slint").unwrap();
    #[cfg(target_os = "windows")]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("../../assets/icons/meridian.ico");
        resource.compile().unwrap();
    }
}
