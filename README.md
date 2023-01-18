# esp-flasher-stub

[![GitHub Workflow Status](https://github.com/esp-rs/esp-println/actions/workflows/ci.yml/badge.svg)](https://github.com/esp-rs/esp-println/actions/workflows/ci.yml)
![MSRV](https://img.shields.io/badge/MSRV-1.60-blue?labelColor=1C2C2E&logo=Rust&style=flat-square)

Rust implementation of flasher stub located in [esptool](https://github.com/espressif/esptool/).

Supports the ESP32, ESP32-C2/C3, and ESP32-S2/S3. Currently `UART` is the only supported transport mode, however support for more is planned.

## Quickstart

In order to build the flasher stub, you must provide a feature to `cargo` selecting the device, and additionally specify the target.

#### ESP32

```
 cargo +esp build --features=esp32 --target=xtensa-esp32-none-elf --release
```

#### ESP32-C2

```
 cargo build --features=esp32c2 --target=riscv32imc-unknown-none-elf --release
```

#### ESP32-C3

```
 cargo build --features=esp32c3 --target=riscv32imc-unknown-none-elf --release
```

#### ESP32-S2

```
 cargo +esp build --features=esp32s2 --target=xtensa-esp32s2-none-elf --release
```

#### ESP32-S3

```
 cargo +esp build --features=esp32s3 --target=xtensa-esp32s3-none-elf --release
```

## Testing

In order to run `test_esptool.py` follow steps below:

- Build `esp-flasher-stub` as described in the build section above.
- Clone `esptool` to the same directory where `esp-flasher-stub` resides.

```
git clone https://github.com/espressif/esptool
```

- Run patched Makefile

```
cd esptool/flasher_stub/
git apply Makefile_patched.patch
make -C .
- Run tests

```
cd ../test
pytest test_esptool.py --port /dev/ttyUSB0 --chip esp32 --baud 115200
```

## Debug logs

In order to use `debug logs` you have to build the project with `dprint` feature, for example:
`cargo build --release --target riscv32imc-unknown-none-elf --features esp32c3,dprint`

and then you can view logs using, for example `screen`:
`sudo screen /dev/ttyUSB2 115200`

> **Warning**
>
> For `ESP32` and `ESP32S2`, please use `baud rate` 57600 instead:
> `sudo screen /dev/ttyUSB2 57600`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
