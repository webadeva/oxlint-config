fn main() {
    // Linker setup for the Node add-on (e.g. dynamic lookup on macOS).
    if std::env::var_os("CARGO_FEATURE_NAPI").is_some() {
        napi_build::setup();
    }
}
