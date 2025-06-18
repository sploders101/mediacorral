use std::sync::Arc;

use mediacorral_proto::mediacorral::coordinator::v1::{
    DiscInsertedRequest, DiscInsertedResponse, RipFinishedRequest, RipFinishedResponse,
    coordinator_notification_service_server::CoordinatorNotificationService,
};

use crate::application::Application;

pub struct NotificationService {
    application: Arc<Application>,
}
impl NotificationService {
    pub fn new(application: Arc<Application>) -> Self {
        return Self { application };
    }
}
#[tonic::async_trait]
impl CoordinatorNotificationService for NotificationService {
    async fn disc_inserted(
        &self,
        request: tonic::Request<DiscInsertedRequest>,
    ) -> Result<tonic::Response<DiscInsertedResponse>, tonic::Status> {
        todo!();
    }
    async fn rip_finished(
        &self,
        request: tonic::Request<RipFinishedRequest>,
    ) -> Result<tonic::Response<RipFinishedResponse>, tonic::Status> {
        todo!();
    }
}
