package analysis

import (
	"encoding/json"
	"os/exec"

	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

type AnalysisController struct {
	analysisCli string
}

func (controller AnalysisController) AnalyzeMkv(path string) (Details, error) {
	cmd := exec.Command(controller.analysisCli, "analyze-mkv", "--", path)
	stdout, err := cmd.Output()
	if err != nil {
		return Details{}, err
	}
	var details Details
	if err := json.Unmarshal(stdout, &details); err != nil {
		return Details{}, err
	}
	return details, nil
}

type Details struct {
	ResolutionWidth  uint32            `json:"resolution_width"`
	ResolutionHeight uint32            `json:"resolution_height"`
	Duration         uint32            `json:"duration"`
	VideoHash        string            `json:"video_hash"`
	Subtitles        *string           `json:"subtitles"`
	ExtendedMetadata *ExtendedMetadata `json:"extended_metadata"`
}
type ExtendedMetadata struct {
	ChapterInfo []ChapterInfo `json:"chapter_info"`
}

func (metadata ExtendedMetadata) IntoProto() *proto.VideoExtendedMetadata {
	meta := proto.VideoExtendedMetadata_builder{}

	var chapterInfo []*proto.ChapterInfo
	for _, chapter := range metadata.ChapterInfo {
		chapterInfo = append(chapterInfo, chapter.IntoProto())
	}

	return meta.Build()
}

type ChapterInfo struct {
	ChapterNumber uint32 `json:"chapter_number"`
	ChapterUid    uint64 `json:"chapter_uid"`
	ChapterStart  uint64 `json:"chapter_start"`
	ChapterEnd    uint64 `json:"chapter_end"`
	ChapterName   string `json:"chapter_name"`
}

func (chapter ChapterInfo) IntoProto() *proto.ChapterInfo {
	return proto.ChapterInfo_builder{
		ChapterNumber: chapter.ChapterNumber,
		ChapterUid:    chapter.ChapterUid,
		ChapterStart:  chapter.ChapterStart,
		ChapterEnd:    chapter.ChapterEnd,
		ChapterName:   chapter.ChapterName,
	}.Build()
}

func NewController(analysisCli string) (AnalysisController, error) {
	if analysisCli == "" {
		analysisCli = "mediacorral-analysis-cli"
	}
	// TODO: Validate analysisCli

	return AnalysisController{
		analysisCli,
	}, nil
}
