package analysis

import (
	proto "github.com/sploders101/mediacorral/backend/gen/mediacorral/server/v1"
)

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
	meta.ChapterInfo = chapterInfo

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

type Subtitle struct {
	Timestamp uint64  `json:"timestamp"`
	Duration  *uint64 `json:"duration"`
	Data      string  `json:"data"`
}
