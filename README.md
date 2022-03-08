# esp-flasher-stub

Rust implementation of flasher stub located in esptool.
Currently only supports ESP32C3 through UART. 

## Build
```
cargo build
```

## Test
```
cargo test --target=x86_64-unknown-linux-gnu
```

## Run esptool test
Since esptool uses precompiled stub binaries located in `stub_flasher.py`, 
binary for ESP32C3 has to be replaced the one otained from `esp-flasher-stub`.

In order to run `test_esptool.py` follow steps below:
* Build `esp-flasher-stub` with `cargo build --release`
* Clone esptool to the same directory where `esp-flasher-stub` resides.
```
git clone https://github.com/espressif/esptool
```
* Navigate to `esptool`, checkout version for which patch located in `esp-flasher-stub` directory was created and apply it.
```
cd esptool
git checkout 6488ebb
git am ../../esp-flasher-stub/esptool.patch
```
* Regenerate `stub_flasher.py` by running patched Makefile and run the tests
```
cd test
make -C ../flasher_stub/ && python test_esptool.py /dev/ttyUSB0 esp32c3 115200
```
This last step requires IDF to be exported.