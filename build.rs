use std::env;
use std::path::PathBuf;

fn main() {
    #[rustfmt::skip]
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .blocklist_item("MS_.*") // one header does #define MS_..., another does
                                 // #undef MS_... and redefines them as enum values -
                                 // this causes bindgen to generate duplicate definitions;
                                 // with this, only the enum values version is generated
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
