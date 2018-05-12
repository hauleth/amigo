# AlcocheMics Image Generator

## Requirements

- [Rust](https://www.rust-lang.org/)
- [Git LFS](https://git-lfs.github.com)

## Fetching

```
git clone https://github.com/hauleth/amigo.git
```

## Building

```
cargo build --release
```

## Running

```
cargo run --release -- --input examples/green.jpg --background examples/bg.jpg --output out.jpg
```

## License

Code is licensed under [ICS License](LICENSE). Other assets (example images) are
be licensed on other terms.
