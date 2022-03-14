use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    #[cfg(target_arch = "riscv32")]
    {
        fs::copy("ld/rom.x", out_dir.join("rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/rom.x");
        println!("cargo:rustc-link-arg=-Tld/rom.x");
    }
}
