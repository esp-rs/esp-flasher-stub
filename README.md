# esp-flasher-stub

Rust implementation of flasher stub located in [esptool](https://github.com/espressif/esptool/).

Currently supports ESP32, ESP32S2, ESP32S3, ESP32C3 and ESP32C2 through UART.

## Build

### ESP32

```
 cargo +esp build --features=esp32 --target=xtensa-esp32-none-elf --release
```

### ESP32S2

```
 cargo +esp build --features=esp32s2 --target=xtensa-esp32s2-none-elf --release
```

### ESP32S3

```
 cargo +esp build --features=esp32s3 --target=xtensa-esp32s3-none-elf --release
```

### ESP32C3

```
 cargo build --features=esp32c3 --target=riscv32imc-unknown-none-elf --release
```

### ESP32C2

```
 cargo build --features=esp32c2 --target=riscv32imc-unknown-none-elf --release
```

## Test

```
cargo test --target=x86_64-unknown-linux-gnu
```

## Run esptool test

Since esptool uses precompiled stub binaries located in `stub_flasher.py`,
binary for ESP32C3 has to be replaced the one otained from `esp-flasher-stub`.

In order to run `test_esptool.py` follow steps below:

- Build `esp-flasher-stub` as described in the build section above.
- Clone esptool to the same directory where `esp-flasher-stub` resides.

```
git clone https://github.com/espressif/esptool
```

- Navigate to `esptool`, checkout version for which patch located in `esp-flasher-stub` directory was created and apply it.

```
cd esptool
git checkout 6488ebb
git am ../../esp-flasher-stub/esptool.patch
```

- Regenerate `stub_flasher.py` by running patched Makefile and run the tests

```
cd test
make -C ../flasher_stub/ && python test_esptool.py /dev/ttyUSB0 esp32c3 115200
```
