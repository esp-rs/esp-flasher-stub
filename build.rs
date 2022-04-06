use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    
    // #[cfg(target_arch = "riscv32")]
    // {
        fs::copy("ld/stub.x", out_dir.join("stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/stub.x");
        println!("cargo:rustc-link-arg=-Tld/stub.x");

        fs::copy("ld/rom.x", out_dir.join("rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/rom.x");
        println!("cargo:rustc-link-arg=-Tld/rom.x");
   // }
}
