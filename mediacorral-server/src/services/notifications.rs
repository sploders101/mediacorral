use std::sync::Arc;

use mediacorral_proto::mediacorral::{
    coordinator::v1::{
        DiscInsertedRequest, DiscInsertedResponse, RipFinishedRequest, RipFinishedResponse,
        coordinator_notification_service_server::CoordinatorNotificationService,
    },
    drive_controller::v1::{GetJobStatusRequest, JobStatus, ReapJobRequest},
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
        let request = request.into_inner();
        if self.application.get_autorip() {
            let _ = self
                .application
                .rip_media(&request.controller_id, request.drive_id, None)
                .await;
        }
        return Ok(tonic::Response::new(DiscInsertedResponse {}));
    }
    async fn rip_finished(
        &self,
        request: tonic::Request<RipFinishedRequest>,
    ) -> Result<tonic::Response<RipFinishedResponse>, tonic::Status> {
        let request = request.into_inner();
        let mut controller = match self
            .application
            .drive_controllers
            .get(&request.controller_id)
        {
            Some(controller) => controller.clone(),
            None => {
                return Ok(tonic::Response::new(RipFinishedResponse {}));
            }
        };
        let job_info = controller
            .get_job_status(GetJobStatusRequest {
                job_id: request.job_id,
            })
            .await?
            .into_inner();
        let status = job_info.status();
        match status {
            JobStatus::Unspecified => {
                println!(
                    concat!(
                        "An error occurred while processing job {}:\n",
                        "  Unknown status: {}",
                        "  Job left on controller for debugging",
                    ),
                    request.job_id, job_info.status
                );
                return Ok(tonic::Response::new(RipFinishedResponse {}));
            }
            JobStatus::Running => {
                println!(
                    "Job {} was reported finished but is still running!",
                    request.job_id
                );
                return Ok(tonic::Response::new(RipFinishedResponse {}));
            }
            JobStatus::Error => {
                // TODO: Record errors so they can be presented in the web UI
                println!(
                    "An error occurred while ripping job {}:{}",
                    request.job_id,
                    job_info
                        .logs
                        .into_iter()
                        .fold(String::new(), |acc, curr| { acc + "\n  " + &curr })
                );
            }
            JobStatus::Completed => {
                if let Err(err) = self.application.import_job(request.job_id).await {
                    println!(
                        "An error occurred while importing job {}:\n{}",
                        request.job_id, err
                    );
                }
            }
        }
        let application = Arc::clone(&self.application);
        let job_id = request.job_id;
        tokio::task::spawn(async move {
            if let Err(err) = application.import_job(job_id).await {
                println!("An error occurred while importing the job:\n{0}", err);
            }
        });
        controller
            .reap_job(ReapJobRequest {
                job_id: request.job_id,
            })
            .await?
            .into_inner();
        return Ok(tonic::Response::new(RipFinishedResponse {}));
    }
}
