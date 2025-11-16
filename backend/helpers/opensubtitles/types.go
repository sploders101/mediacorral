package opensubtitles

import "github.com/sploders101/mediacorral/backend/dbapi"

type ostSearchResults struct {
	Data []struct {
		Attributes struct {
			Language         *string     `json:"language"`
			DownloadCount    uint32      `json:"download_count"`
			NewDownloadCount uint32      `json:"new_download_count"`
			Uploader         OstUploader `json:"uploader"`
			Files            []struct {
				FileId   uint32 `json:"file_id"`
				FileName string `json:"file_name"`
			} `json:"files"`
		} `json:"attributes"`
	} `json:"data"`
}

type SubtitleSummary struct {
	Name             string
	FileId           uint32
	DownloadCount    uint32
	NewDownloadCount uint32
	Uploader         OstUploader
}

type OstUploader struct {
	Name string `json:"name"`
	Rank string `json:"rank"`
}

type GetSubtitlesResult struct {
	SubtitlesItem dbapi.OstDownloadsItem
	Subtitles     string
}

type FindBestSubtitlesResult struct {
	Filename          string
	Subtitles         string
	minifiedSubtitles string
}
