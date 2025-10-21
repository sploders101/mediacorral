pub mod mediacorral {
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("mediacorral");
    pub mod drive_controller {
        pub mod v1 {
            tonic::include_proto!("mediacorral.drive_controller.v1");
        }
    }
    pub mod server {
        pub mod v1 {
            tonic::include_proto!("mediacorral.server.v1");
        }
    }
}
