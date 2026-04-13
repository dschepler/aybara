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

    let libnl_route_lib = pkg_config::Config::new()
        .atleast_version("3.12.0")
        .probe("libnl-route-3.0")
        .expect("libnl-route library is required");

    let libnl_bindings = bindgen::Builder::default()
        .header("libnl_wrapper.h")
        .blocklist_item("IPPORT_RESERVED")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(
            libnl_route_lib
                .include_paths
                .iter()
                .map(|p| format!("-I{}", p.display())),
        )
        .generate()
        .expect("Unable to generate libnl bindings");

    // Write the bindings to the $OUT_DIR/libnl_bindings.rs file.
    libnl_bindings
        .write_to_file(out_path.join("libnl_bindings.rs"))
        .expect("Bouldn't write libnl bindings");
}
