use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out_dir.display());

    #[cfg(feature = "esp32")]
    {
        fs::copy("ld/esp32_stub.x", out_dir.join("esp32_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32_stub.x");

        fs::copy("ld/esp32_rom.x", out_dir.join("esp32_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32_rom.x");
    }

    #[cfg(feature = "esp32c2")]
    {
        fs::copy("ld/esp32c2_stub.x", out_dir.join("esp32c2_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32c2_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c2_stub.x");

        fs::copy("ld/esp32c2_rom.x", out_dir.join("esp32c2_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32c2_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c2_rom.x");

        println!("cargo:rustc-link-arg=-Thal-defaults.x");
    }

    #[cfg(feature = "esp32c3")]
    {
        fs::copy("ld/esp32c3_stub.x", out_dir.join("esp32c3_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32c3_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c3_stub.x");

        fs::copy("ld/esp32c3_rom.x", out_dir.join("esp32c3_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32c3_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c3_rom.x");

        println!("cargo:rustc-link-arg=-Thal-defaults.x");
    }

    #[cfg(feature = "esp32c6")]
    {
        fs::copy("ld/esp32c6_stub.x", out_dir.join("esp32c6_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32c6_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c6_stub.x");

        fs::copy("ld/esp32c6_rom.x", out_dir.join("esp32c6_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32c6_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32c6_rom.x");

        println!("cargo:rustc-link-arg=-Thal-defaults.x");
    }

    #[cfg(feature = "esp32h2")]
    {
        fs::copy("ld/esp32h2_stub.x", out_dir.join("esp32h2_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32h2_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32h2_stub.x");

        fs::copy("ld/esp32h2_rom.x", out_dir.join("esp32h2_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32h2_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32h2_rom.x");

        println!("cargo:rustc-link-arg=-Thal-defaults.x");
    }

    #[cfg(feature = "esp32s2")]
    {
        fs::copy("ld/esp32s2_stub.x", out_dir.join("esp32s2_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32s2_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32s2_stub.x");

        fs::copy("ld/esp32s2_rom.x", out_dir.join("esp32s2_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32s2_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32s2_rom.x");
    }

    #[cfg(feature = "esp32s3")]
    {
        fs::copy("ld/esp32s3_stub.x", out_dir.join("esp32s3_stub.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/esp32s3_stub.x");
        println!("cargo:rustc-link-arg=-Tld/esp32s3_stub.x");

        fs::copy("ld/esp32s3_rom.x", out_dir.join("esp32s3_rom.x")).unwrap();
        println!("cargo:rerun-if-changed=ld/ld/esp32s3_rom.x");
        println!("cargo:rustc-link-arg=-Tld/esp32s3_rom.x");
    }

    emit_cfg();
}

fn emit_cfg() {
    #[cfg(any(
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s3",
    ))]
    println!("cargo:rustc-cfg=usb_device");

    #[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
    println!("cargo:rustc-cfg=usb0");
}
