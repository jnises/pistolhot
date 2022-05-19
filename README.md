# Pistolhot

Work in progress double pendulum synth

![screenshot](docs/screenshot.jpg)

## To build and run natively:
```
cargo run --release -p pistolhot-standalone
```

## To build vst
```
cd vst
# on windows
cargo build
# on mac
cargo make mac_bundle_debug
```

## To build wasm version for web.
> :warning: this target doesn't currently work
```
cargo install cargo-make
cd wasm

# option 1
cargo make build_web
# option 2
cargo make watch

# in separate shell
cargo make serve
```
 
The open a browser (with webmidi support) and point it to http://localhost:8000