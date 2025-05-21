package timeutils

import (
	"fmt"
	"strings"
)

// FormatSecondsToHHMMSS converts total seconds to HH:MM:SS format.
func FormatSecondsToHHMMSS(totalSeconds int) string {
	hours := totalSeconds / 3600
	minutes := (totalSeconds % 3600) / 60
	seconds := totalSeconds % 60
	return fmt.Sprintf("%02d:%02d:%02d", hours, minutes, seconds)
}

// ParseHHMMSSToSeconds converts HH:MM:SS string to total seconds.
func ParseHHMMSSToSeconds(timeStr string) (int, error) {
	parts := strings.Split(timeStr, ":")
	if len(parts) != 3 {
		return 0, fmt.Errorf("invalid time format: expected HH:MM:SS, got %s", timeStr)
	}

	var h, m, s int
	// Using Sscanf to parse the integers directly from the string
	_, err := fmt.Sscanf(timeStr, "%d:%d:%d", &h, &m, &s)
	if err != nil {
		return 0, fmt.Errorf("error parsing time string '%s': %w", timeStr, err)
	}

	if h < 0 || m < 0 || m > 59 || s < 0 || s > 59 {
		return 0, fmt.Errorf("invalid time values in %s: hours, minutes, or seconds out of range", timeStr)
	}
	return (h * 3600) + (m * 60) + s, nil
}
