package afk

import (
	"bytes"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
)

// getIdleTimeMillis executes the `xprintidle` command and returns the idle time in milliseconds.
func getIdleTimeMillis() (int64, error) {
	cmd := exec.Command("xprintidle")
	var out bytes.Buffer
	cmd.Stdout = &out
	err := cmd.Run()
	if err != nil {
		return 0, fmt.Errorf("failed to execute xprintidle: %w (is xprintidle installed?)", err)
	}
	idleTimeStr := strings.TrimSpace(out.String())
	idleTimeMillis, err := strconv.ParseInt(idleTimeStr, 10, 64)
	if err != nil {
		return 0, fmt.Errorf("failed to parse idle time '%s': %w", idleTimeStr, err)
	}
	return idleTimeMillis, nil
}

// isAudioPlaying checks if any audio sink input is currently in a "RUNNING" state.
// This is a common way to check for active audio playback on systems using PulseAudio/PipeWire.
func isAudioPlaying() (bool, error) {
	// The command checks all sink inputs for a "state" line that indicates "RUNNING".
	// Using `grep -E` for a slightly more robust pattern matching.
	// `grep -c` counts matching lines. If count > 0, audio is playing.
	cmdStr := "pacmd list-sink-inputs | grep -E '^[[:space:]]*state:[[:space:]]*RUNNING'"
	cmd := exec.Command("sh", "-c", cmdStr) // Use sh -c to handle pipes

	var out bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &stderr

	err := cmd.Run()

	// Grep exits with status 1 if no lines were selected (no audio playing),
	// and with 0 if lines were selected (audio playing).
	// Other exit codes indicate an error with grep itself or pacmd.
	if err != nil {
		// If err is ExitError and no output, it means grep found no matches (no audio)
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 1 && strings.TrimSpace(out.String()) == "" {
				return false, nil // No running audio sinks found
			}
		}
		// For other errors, or if grep failed for another reason
		return false, fmt.Errorf("failed to check audio state (pacmd/grep command failed): %w. Stderr: %s", err, stderr.String())
	}

	// If command ran successfully (exit code 0), it means grep found matching lines.
	return strings.TrimSpace(out.String()) != "", nil
}

// IsUserAFK checks if the user is AFK based on idle time and audio playback.
// timeoutMillis: The duration in milliseconds after which a user is considered idle.
func IsUserAFK(timeoutMillis int64) (bool, error) {
	idleTime, err := getIdleTimeMillis()
	if err != nil {
		return false, fmt.Errorf("failed to get idle time: %w", err)
	}

	if idleTime > timeoutMillis {
		// User has been idle for longer than the timeout.
		// Check if audio is playing to determine if truly AFK.
		audioPlaying, err := isAudioPlaying()
		if err != nil {
			// If checking audio fails, err on the side of caution: assume user is not AFK.
			return false, fmt.Errorf("failed to check audio playing status: %w (assuming not AFK)", err)
		}
		if audioPlaying {
			return false, nil // Idle, but audio is playing, so not considered AFK.
		}
		return true, nil // Idle and no audio, so user is AFK.
	}

	// Idle time is less than or equal to the timeout, so user is not AFK.
	return false, nil
}
