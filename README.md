# esp-flasher-stub

Rust implementation of flasher stub located in esptool.
Currently only supports ESP32C3 and ESP32 through UART. 

## Build
```
 cargo build --features esp32 --target xtensa-esp32-none-elf --release
```
 or
```
 cargo build --features esp32c3 --target riscv32imc-unknown-none-elf --release
```

## Test
cargo test --target=x86_64-unknown-linux-gnu

## Run esptool test
Since esptool uses precompiled stub binaries located in `stub_flasher.py`, 
binary for ESP32C3 has to be replaced the one otained from `esp-flasher-stub`.

In order to run `test_espttol.py` follow steps below:
* Build `esp-flasher-stub` as described in build section above.
* Clone esptool to the same directory where `esp-flasher-stub` resides.
```
git clone https://github.com/espressif/esptool
```
* Navigate to `esptool/test` and apply patch located in `esp-flasher-stub` directory.
```
cd esptool/test
git am ../../esp-flasher-stub/esptool.patch
```
* Regenerate `stub_flasher.py` by running patched Makefile and run the tests
```
make -C ../flasher_stub/ && python test_esptool.py /dev/ttyUSB0 esp32c3 115200
```