fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/app.rc");
    println!("cargo:rerun-if-changed=resources/app.ico");

    #[cfg(target_os = "windows")]
    {
        let resource_dir = std::path::Path::new("resources");
        let rc_file = resource_dir.join("app.rc");
        let _ = embed_resource::compile(rc_file, embed_resource::ParamsIncludeDirs([resource_dir]));
    }
}
