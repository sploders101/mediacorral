use std::sync::Arc;

use mediacorral_proto::coordinator::{
    JobReplyComputeDistanceRequest, JobReplyOcrSubtitlesRequest, JobReplyResponse, JobRequest,
    JobResponse, coordinator_job_service_server::CoordinatorJobService,
};

use crate::application::Application;

pub struct JobService {
    application: Arc<Application>,
}
impl JobService {
    pub fn new(application: Arc<Application>) -> Self {
        return Self { application };
    }
}
#[tonic::async_trait]
impl CoordinatorJobService for JobService {
    async fn request_job(
        &self,
        request: tonic::Request<JobRequest>,
    ) -> Result<tonic::Response<JobResponse>, tonic::Status> {
        todo!();
    }
    async fn job_reply_compute_distance(
        &self,
        request: tonic::Request<JobReplyComputeDistanceRequest>,
    ) -> Result<tonic::Response<JobReplyResponse>, tonic::Status> {
        todo!();
    }
    async fn job_reply_ocr_subtitles(
        &self,
        request: tonic::Request<JobReplyOcrSubtitlesRequest>,
    ) -> Result<tonic::Response<JobReplyResponse>, tonic::Status> {
        todo!();
    }
}
