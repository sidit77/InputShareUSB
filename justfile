target := "arm-unknown-linux-gnueabihf"
# target := armv7-unknown-linux-gnueabihf

remote := "pi@raspberrypi.local:"

default:
  @just --choose

check-server:
    cargo check --bin inputshare-server

build-server:
    cross build --bin inputshare-server --target={{target}} --release

upload-binary:
    scp ./target/{{target}}/release/inputshare-server {{remote}}

build-upload: build-server upload-binary

sync-wsl:
    @rsync --out-format='updating %n' --include='**.gitignore' --exclude='/.git' --filter=':- .gitignore' --delete-after -hra . ~/InputShareUSB

build-server-wsl: sync-wsl
    cd ~/InputShareUSB && cross build --bin inputshare-server --target={{target}} --release

upload-binary-wsl:
    scp ~/InputShareUSB/target/{{target}}/release/inputshare-server {{remote}}

build-upload-wsl: build-server-wsl upload-binary-wsl

flash-esp: sync-wsl
    cd ~/InputShareUSB && cargo +nightly run --bin inputshare-esp32 --target riscv32imc-esp-espidf --profile release-esp