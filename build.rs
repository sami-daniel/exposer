use bindgen;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    // Tricky, butttt, we need to impl some traits via derive macros
    // to bytemuck works correctly, so we need to disable this one

    // let bindings = bindgen::Builder::default()
    //     .header("wrapper.h")
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
    //     .generate()
    //     .expect("Unable to generate bindings");

    // bindings
    //     .write_to_file("./src/types.rs")
    //     .expect("Error while trying to write to types.rs");
}
