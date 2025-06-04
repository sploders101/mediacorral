pub fn main() {
    tonic_build::compile_protos("proto/drive_controller.proto").expect("Couldn't compile protos.");
    tonic_build::compile_protos("proto/coordinator.proto").expect("Couldn't compile protos.");
}
