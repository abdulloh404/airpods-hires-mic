fn main() {
    pkg_config::Config::new()
        .atleast_version("2.0")
        .probe("fdk-aac")
        .expect("libfdk-aac development files are required (Ubuntu/Debian: libfdk-aac-dev)");
}
