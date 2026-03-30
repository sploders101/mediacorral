package drive_coordinator

import (
	"os"

	drive_coordinatorv1 "github.com/sploders101/mediacorral/backend/gen/mediacorral/drive_coordinator/v1"
)

func getFinishedFiles(
	ripDir string,
	files []*drive_coordinatorv1.FileDescription,
) ([]*drive_coordinatorv1.FileDescription, error) {
	existingFiles, err := os.ReadDir(ripDir)
	if err != nil {
		return nil, err
	}

	isUploading := func(fileName string) bool {
		for _, file := range files {
			if file.GetFileName() == fileName {
				return true
			}
		}
		return false
	}

	var finishedFiles []*drive_coordinatorv1.FileDescription
	for _, fileEntry := range existingFiles {
		if !fileEntry.Type().IsRegular() || isUploading(fileEntry.Name()) {
			continue
		}
		fileInfo, err := fileEntry.Info()
		if err != nil {
			return nil, err
		}
		finishedFiles = append(finishedFiles, drive_coordinatorv1.FileDescription_builder{
			FileName: fileEntry.Name(),
			FileSize: uint64(fileInfo.Size()),
		}.Build())
	}

	return finishedFiles, nil
}
