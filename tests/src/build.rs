extern crate glob;
extern crate lcm_gen;

fn main() {
    let files: Vec<_> = glob::glob("lcm/*.lcm")
        .expect("Failed to find LCM files")
        .filter_map(Result::ok)
        .collect();

    lcm_gen::generate(&files).expect("Failed to generate bindings for LCM types");
}
