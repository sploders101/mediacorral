package application

func (app *Application) importJobs() {
	for {
		ripJob := <-app.DriveCoordinator.ImportQueue
		go app.ImportJob(ripJob)
	}
}
