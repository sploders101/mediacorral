use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("Missing OUT_DIR"));
    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("mediacorral.bin"))
        .compile_protos(
            &["../proto/mediacorral/analysis/v1/main.proto"],
            &["../proto"],
        )?;

    return Ok(());
}
