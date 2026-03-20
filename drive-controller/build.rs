pub fn main() {
    let protoc_bin = protoc_bin_vendored::protoc_bin_path().unwrap();
    unsafe { std::env::set_var("PROTOC", protoc_bin) };

    tonic_build::configure()
        .compile_protos(
            &["../proto/mediacorral/drive_coordinator/v1/main.proto"],
            &["../proto/"],
        )
        .unwrap();
}
