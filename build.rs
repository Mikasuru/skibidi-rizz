fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-search=native=libs/x64");
        println!("cargo:rustc-link-lib=static=Packet");
    }
}
