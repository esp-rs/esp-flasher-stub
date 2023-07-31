# esp-flasher-stub

[![GitHub Workflow Status](https://github.com/esp-rs/esp-println/actions/workflows/ci.yml/badge.svg)](https://github.com/esp-rs/esp-println/actions/workflows/ci.yml)
![MSRV](https://img.shields.io/badge/MSRV-1.65-blue?labelColor=1C2C2E&logo=Rust&style=flat-square)
[![Matrix](https://img.shields.io/matrix/esp-rs:matrix.org?label=join%20matrix&color=BEC5C9&labelColor=1C2C2E&logo=matrix&style=flat-square)](https://matrix.to/#/#esp-rs:matrix.org)

Rust implementation of flasher stub located in [esptool](https://github.com/espressif/esptool/).

Supports the ESP32, ESP32-C2/C3/C6, ESP32-H2, and ESP32-S2/S3. Currently `UART` and `USB Serial JTAG` are the supported transport modes, and support for other modes is planned.

## Quickstart

To ease the building process we have included a `build` subcommand in the `xtask` package which will apply all the appropriate build configuration for one or more devices:

```bash
cargo xtask build esp32
cargo xtask build esp32c2 esp32c3
```

In order to build the flasher stub manually, you must specify the appropriate toolchain, provide a feature to `cargo` selecting the device, and additionally specify the target:

```bash
# ESP32
cargo +esp build --release --features=esp32 --target=xtensa-esp32-none-elf

# ESP32-C2
cargo +nightly build --release --features=esp32c2 --target=riscv32imc-unknown-none-elf

# ESP32-C3
cargo +nightly build --release --features=esp32c3 --target=riscv32imc-unknown-none-elf

# ESP32-C6
cargo +nightly build --release --features=esp32c6 --target=riscv32imac-unknown-none-elf

# ESP32-H2
cargo +nightly build --release --features=esp32h2 --target=riscv32imac-unknown-none-elf

# ESP32-S2
cargo +esp build --release --features=esp32s2 --target=xtensa-esp32s2-none-elf

# ESP32-S3
cargo +esp build --release --features=esp32s3 --target=xtensa-esp32s3-none-elf
```

In order to generate the JSON stub files for one or more devices, you can again use the `xtask` package:

```bash
cargo xtask wrap esp32c3
cargo xtask wrap esp32 esp32s2 esp32s3
```

## Testing

In order to run `test_esptool.py` follow steps below:

- Build `esp-flasher-stub` as described in the section above.
- Clone `esptool` to the same directory where `esp-flasher-stub` resides.
- Run patched Makefile
- Run tests

```bash
git clone https://github.com/espressif/esptool
cd esptool/flasher_stub/
git apply Makefile_patched.patch
make -C .
cd ../test
pytest test_esptool.py --port /dev/ttyUSB0 --chip esp32 --baud 115200
```

## Debug logs

In order to use debug logs you have to build the project with `dprint` feature, for example:

```bash
cargo build --release --target=riscv32imc-unknown-none-elf --features=esp32c3,dprint
```

Then you can view logs using, for example `screen`:

```bash
screen /dev/ttyUSB2 115200
```

> **Warning**
>
> For ESP32 and ESP32-S2, please use a baud rate of 57,600 instead:
>
> ```bash
> screen /dev/ttyUSB2 57600
> ```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
