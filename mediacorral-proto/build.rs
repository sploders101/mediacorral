pub fn main() {
    tonic_build::compile_protos("proto/ripper.proto").expect("Couldn't compile protos.");
}
