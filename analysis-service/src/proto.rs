pub mod mediacorral {
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("mediacorral");
    pub mod analysis {
        pub mod v1 {
            tonic::include_proto!("mediacorral.analysis.v1");
        }
    }
}
