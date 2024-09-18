# esp-flasher-stub

[![GitHub Workflow Status](https://github.com/esp-rs/esp-flasher-stub/actions/workflows/ci.yml/badge.svg)](https://github.com/esp-rs/esp-flasher-stub/actions/workflows/ci.yml)
![MSRV](https://img.shields.io/badge/MSRV-1.76-blue?labelColor=1C2C2E&logo=Rust&style=flat-square)
[![Matrix](https://img.shields.io/matrix/esp-rs:matrix.org?label=join%20matrix&color=BEC5C9&labelColor=1C2C2E&logo=matrix&style=flat-square)](https://matrix.to/#/#esp-rs:matrix.org)

Rust implementation of the [esptool flasher stub](https://github.com/espressif/esptool-legacy-flasher-stub/).

Supports the ESP32, ESP32-C2/C3/C6, ESP32-H2, and ESP32-S2/S3. Currently, `UART` and `USB Serial JTAG` are the supported transport modes, and support for other modes is planned.

## Quickstart

To ease the building process we have included a `build` subcommand in the `xtask` package which will apply all the appropriate build configurations for one or more devices:

```bash
cd xtask/
cargo run -- build esp32
cargo run -- build esp32c2 esp32c3
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

In order to generate the JSON and TOML stub files for one or more devices, you can again use the `xtask` package:

```bash
cd xtask/
cargo run -- wrap esp32c3
cargo run -- wrap esp32 esp32s2 esp32s3
```

JSON stub files will be generated in the project root directory.

## Testing

In order to run `test_esptool.py` follow steps below:

- Build `esp-flasher-stub` as described in the section above.
- Clone `esptool`, if you don't have it yet:
  ```
  git clone https://github.com/espressif/esptool
  ```
- Copy the stub JSON files into `esptool` installation. You can use the following one-liner:
  ```bash
  for n in esp*.json; do cp $n $ESPTOOL_PATH/esptool/targets/stub_flasher/2/$n; done
  ```
  where `ESPTOOL_PATH` is set to the location where you have cloned `esptool`.
- Set `ESPTOOL_STUB_VERSION` environment variable to `2`.
- Run tests
  ```bash
  cd $ESPTOOL_PATH/test
  pytest test_esptool.py --port /dev/ttyUSB0 --chip esp32 --baud 115200
  ```

## Debug logs

In order to add debug logs, you can use the `--dprint` flag available in the `xtask` package for `build` and `wrap` commands:
```bash
cd xtask/
cargo run -- wrap esp32c3 --dprint
cargo run -- build esp32 esp32s2 esp32s3 --dprint
```

In order to add debug logs when building the flasher stub manually you have to build the project with `dprint` feature, for example:

```bash
cargo build --release --target=riscv32imc-unknown-none-elf --features=esp32c3,dprint
```

This will print `esp-flasher-stub` debug messages using `UART1`. By default, `esp-flasher-stub` uses the following pins:
- TX: GPIO 2
- RX: GPIO 0

Then you can view logs using, for example, `screen`:

```bash
screen /dev/ttyUSB2 115200
```

> [!WARNING]
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
