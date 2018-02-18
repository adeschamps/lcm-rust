# LCM Code Generator

This tool generates Rust code from LCM schema files. It can be used as part of a build.rs file or as a standalone binary.

## As part of a build

To generate LCM bindings as part of the build process of your project, add `lcm-gen` as a dependency:

```toml
[build-dependencies]
lcm-gen = { version = "0.2.0", default-features = false }
```

... and then invoke it from a `build.rs` script:

```rust
extern crate lcm_gen;

fn main() {
    lcm_gen::Config::default()
        .generate(&[
            "lcm/point_t.lcm",
            "lcm/temperature_t.lcm",
        ])
        .expect("Failed to generate LCM bindings");
}
```

## As a standalone binary

The tool can also be built as a command line tool:

```bash
$ cargo install
$ lcm-gen-rust lcm/temperature_t.lcm
```
