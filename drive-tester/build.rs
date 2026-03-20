pub fn main() {
    tonic_build::configure()
        .compile_protos(
            &["../proto/mediacorral/drive_coordinator/v1/main.proto"],
            &["../proto/"],
        )
        .unwrap();
}
