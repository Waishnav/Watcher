package timeutils

import (
	"fmt"
	"testing"
)

func TestFormatSecondsToHHMMSS(t *testing.T) {
	tests := []struct {
		name     string
		seconds  int
		expected string
	}{
		{"zero seconds", 0, "00:00:00"},
		{"few seconds", 5, "00:00:05"},
		{"one minute", 60, "00:01:00"},
		{"minutes and seconds", 125, "00:02:05"},
		{"one hour", 3600, "01:00:00"},
		{"hours, minutes, seconds", 3661, "01:01:01"},
		{"large hours", 86400, "24:00:00"}, // 24 hours
		{"very large hours", 90000, "25:00:00"}, // 25 hours
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := FormatSecondsToHHMMSS(tt.seconds); got != tt.expected {
				t.Errorf("FormatSecondsToHHMMSS(%d) = %q, want %q", tt.seconds, got, tt.expected)
			}
		})
	}
}

func TestParseHHMMSSToSeconds(t *testing.T) {
	tests := []struct {
		name      string
		timeStr   string
		expected  int
		expectErr bool
	}{
		{"zero time", "00:00:00", 0, false},
		{"only seconds", "00:00:30", 30, false},
		{"only minutes", "00:05:00", 300, false},
		{"minutes and seconds", "00:02:30", 150, false},
		{"only hours", "02:00:00", 7200, false},
		{"hours, minutes, seconds", "01:01:01", 3661, false},
		{"large hours", "25:00:00", 90000, false},
		{"max values", "23:59:59", 86399, false},

		// Error cases
		{"invalid format short", "1:2:3", 0, true},
		{"invalid format letters", "aa:bb:cc", 0, true},
		{"missing parts", "01:02", 0, true},
		{"too many parts", "01:02:03:04", 0, true},
		{"empty string", "", 0, true},
		{"seconds out of range", "00:00:60", 0, true},
		{"minutes out of range", "00:60:00", 0, true},
		{"negative hours", "-01:00:00", 0, true}, // sscanf might parse -1, but our validation should catch it
		{"non-numeric", "00:aa:00", 0, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := ParseHHMMSSToSeconds(tt.timeStr)
			if (err != nil) != tt.expectErr {
				t.Errorf("ParseHHMMSSToSeconds(%q) error = %v, expectErr %v", tt.timeStr, err, tt.expectErr)
				return
			}
			if !tt.expectErr && got != tt.expected {
				t.Errorf("ParseHHMMSSToSeconds(%q) = %d, want %d", tt.timeStr, got, tt.expected)
			}
		})
	}
}
