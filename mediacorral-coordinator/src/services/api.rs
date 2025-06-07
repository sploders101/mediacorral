use std::sync::Arc;

use mediacorral_proto::coordinator::coordinator_api_service_server::CoordinatorApiService;

use crate::application::Application;

pub struct ApiService {
    application: Arc<Application>,
}
impl ApiService {
    pub fn new(application: Arc<Application>) -> Self {
        return Self { application };
    }
}
#[tonic::async_trait]
impl CoordinatorApiService for ApiService {}
