// Give the cdylib an @rpath install name instead of cargo's default absolute
// build-tree path. An executable that links the dylib then records
// `@rpath/libswiftflow_desktop.dylib` as its dependency and finds it through an
// rpath the app's link line adds (see Sources/CSwiftFlow in ../../Package.swift)
// — so the framework works from a fresh shell, wherever it's installed, with no
// DYLD_LIBRARY_PATH. Applies only to the cdylib artifact; the staticlib and
// rlib are untouched.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!(
            "cargo::rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libswiftflow_desktop.dylib"
        );
    }
}
