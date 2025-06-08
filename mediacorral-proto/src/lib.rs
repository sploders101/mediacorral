pub mod mediacorral {
    pub mod drive_controller {
        pub mod v1 {
            tonic::include_proto!("mediacorral.drive_controller.v1");
        }
    }
    pub mod coordinator {
        pub mod v1 {
            tonic::include_proto!("mediacorral.coordinator.v1");
        }
    }
    pub mod common {
        pub mod tmdb {
            pub mod v1 {
                tonic::include_proto!("mediacorral.common.tmdb.v1");
            }
        }
    }
}
