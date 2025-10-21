use std::path::PathBuf;

pub fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("Missing OUT_DIR"));
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("mediacorral.bin"))
        .compile_protos(
            &[
                "../proto/mediacorral/drive_controller/v1/main.proto",
                "../proto/mediacorral/server/v1/notifications.proto",
            ],
            &["../proto/"],
        )
        .unwrap();
}
