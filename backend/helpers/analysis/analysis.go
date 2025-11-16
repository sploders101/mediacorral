package analysis

import (
	"bytes"
	"encoding/json"
	"os/exec"
)

type AnalysisController struct {
	analysisCli string
}

func (controller *AnalysisController) AnalyzeMkv(path string) (Details, error) {
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

func (controller *AnalysisController) ParseSrt(srtText string) ([]Subtitle, error) {
	cmd := exec.Command(controller.analysisCli, "srt2json", "--", "-")
	cmd.Stdin = bytes.NewReader([]byte(srtText))
	stdout, err := cmd.Output()
	if err != nil {
		return nil, err
	}
	var subtitles []Subtitle
	if err := json.Unmarshal(stdout, &subtitles); err != nil {
		return nil, err
	}
	return subtitles, err
}

func (controller *AnalysisController) ParseSrtFile(filePath string) ([]Subtitle, error) {
	cmd := exec.Command(controller.analysisCli, "srt2json", "--", filePath)
	stdout, err := cmd.Output()
	if err != nil {
		return nil, err
	}
	var subtitles []Subtitle
	if err := json.Unmarshal(stdout, &subtitles); err != nil {
		return nil, err
	}
	return subtitles, err
}

func NewController(analysisCli string) (*AnalysisController, error) {
	if analysisCli == "" {
		analysisCli = "mediacorral-analysis-cli"
	}
	// TODO: Validate analysisCli

	return &AnalysisController{
		analysisCli,
	}, nil
}
