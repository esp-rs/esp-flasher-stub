use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search)={}", out.display());

    let chip = if cfg!(feature = "esp32") {
        "esp32"
    } else if cfg!(feature = "esp32c2") {
        "esp32c2"
    } else if cfg!(feature = "esp32c3") {
        "esp32c3"
    } else if cfg!(feature = "esp32c6") {
        "esp32c6"
    } else if cfg!(feature = "esp32h2") {
        "esp32h2"
    } else if cfg!(feature = "esp32s2") {
        "esp32s2"
    } else if cfg!(feature = "esp32s3") {
        "esp32s3"
    } else {
        panic!("Must select exactly one chip feature!")
    };

    let arch = match chip {
        "esp32" | "esp32s2" | "esp32s3" => "xtensa",
        _ => "riscv32",
    };

    // Define configuration symbols:

    println!("cargo:rustc-cfg={chip}");
    println!("cargo:rustc-cfg={arch}");

    // Define any USB-related configuration symbols, if required:

    if cfg!(feature = "esp32c3")
        || cfg!(feature = "esp32c6")
        || cfg!(feature = "esp32h2")
        || cfg!(feature = "esp32s3")
    {
        println!("cargo:rustc-cfg=usb_device");
    }

    if cfg!(feature = "esp32s2") || cfg!(feature = "esp32s3") {
        println!("cargo:rustc-cfg=usb0");
    }

    // Copy required linker scripts to the `out` path:

    let ld_path = PathBuf::from("ld");

    let stub_x = format!("{}_stub.x", chip);
    fs::copy(ld_path.join(&stub_x), out.join(&stub_x))?;
    println!("cargo:rerun-if-changed=ld/{stub_x}");
    println!("cargo:rustc-link-arg=-Tld/{stub_x}");

    let rom_x = format!("{}_rom.x", chip);
    fs::copy(ld_path.join(&rom_x), out.join(&rom_x))?;
    println!("cargo:rerun-if-changed=ld/{rom_x}");
    println!("cargo:rustc-link-arg=-Tld/{rom_x}");

    // The RISC-V devices additionally require the `hal-defaults.x` linker
    // script from `esp-hal`, to avoid interrupt-related linker errors:

    if arch == "riscv32" {
        println!("cargo:rustc-link-arg=-Thal-defaults.x");
    }

    // Done!

    Ok(())
}
