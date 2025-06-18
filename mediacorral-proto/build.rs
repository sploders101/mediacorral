pub fn main() {
    tonic_build::configure()
        .compile_protos(
            &[
                "proto/mediacorral/common/tmdb/v1/main.proto",
                "proto/mediacorral/drive_controller/v1/main.proto",
                "proto/mediacorral/coordinator/v1/api.proto",
                "proto/mediacorral/coordinator/v1/notifications.proto",
            ],
            &["proto/"],
        )
        .unwrap();
}
