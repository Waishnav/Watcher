package watcher

import (
	"bytes"
	"encoding/csv"
	"fmt"
	"io"
	"os"
	"os/exec"
	"os/user"
	"path/filepath"
	"strings"
	"time"
	"watcher/pkg/afk"      // Import for AFK detection
	"watcher/pkg/timeutils" // Import for time utilities
)

const (
	// afkTimeoutMillis defines the time in milliseconds after which a user is considered idle.
	// For example, 2 minutes = 2 * 60 * 1000 = 120000 ms.
	// Let's use 1 minute for easier testing: 1 * 60 * 1000 = 60000 ms
	afkTimeoutMillis = 60000
)

// getActiveWindowClassName executes xdotool and xprop to find the WM_CLASS of the currently focused window.
func getActiveWindowClassName() (string, error) {
	// Get focused window ID
	cmdGetFocus := exec.Command("xdotool", "getwindowfocus")
	var outFocus bytes.Buffer
	cmdGetFocus.Stdout = &outFocus
	err := cmdGetFocus.Run()
	if err != nil {
		return "", fmt.Errorf("failed to get focused window: %w (is xdotool installed?)", err)
	}
	windowID := strings.TrimSpace(outFocus.String())

	// Get WM_CLASS using window ID
	cmdGetClass := exec.Command("xprop", "-id", windowID, "WM_CLASS")
	var outClass bytes.Buffer
	cmdGetClass.Stdout = &outClass
	err = cmdGetClass.Run()
	if err != nil {
		// It's possible for xprop to fail if the window closes between calls.
		// We can treat this as "no specific window focused" or a transient error.
		return "", fmt.Errorf("failed to get WM_CLASS for window ID %s: %w", windowID, err)
	}

	// Parse WM_CLASS output
	// Example output: WM_CLASS(STRING) = "Navigator", "Firefox"
	// Or: WM_CLASS(STRING) = "google-chrome", "Google-chrome"
	// We want the second string in quotes, or the first if only one is present.
	output := outClass.String()
	parts := strings.Split(output, "=")
	if len(parts) < 2 {
		return "", fmt.Errorf("unexpected WM_CLASS format: %s", output)
	}

	valuePart := strings.TrimSpace(parts[1])
	values := strings.Split(valuePart, ",")
	if len(values) == 0 {
		return "", fmt.Errorf("no WM_CLASS values found: %s", output)
	}

	// Get the last class name, remove quotes and trim spaces
	// For "Navigator", "Firefox", it takes "Firefox"
	// For "google-chrome", "Google-chrome", it takes "Google-chrome"
	// For "Alacritty", "Alacritty", it takes "Alacritty"
	className := strings.TrimSpace(values[len(values)-1])
	className = strings.Trim(className, "\"")

	if className == "" {
		// This can happen for some windows (e.g., desktop itself or panels)
		return "Unknown/Desktop", nil
	}

	return className, nil
}

// getDailyLogFilePath constructs the path for the daily log file.
// baseCacheDir should be like "/path/to/user/.cache/Watcher"
func getDailyLogFilePath(baseCacheDir string, dateStr string) (string, error) {
	if baseCacheDir == "" {
		return "", fmt.Errorf("baseCacheDir cannot be empty")
	}
	dailyDataDir := filepath.Join(baseCacheDir, "daily_data")
	// Ensure the directory exists before returning path (important for when writeDailyActivity uses it)
	// However, for loading, the directory might not exist yet.
	// Let writeDailyActivity handle directory creation for its use case.
	return filepath.Join(dailyDataDir, dateStr+".csv"), nil
}

// loadDailyActivity loads the activity data from the CSV file for the given date.
// filePath is the full path to the log file.
func loadDailyActivity(filePath string, activity map[string]int) error {
	file, err := os.Open(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil // File doesn't exist, so no data to load
		}
		return fmt.Errorf("failed to open daily log file %s: %w", filePath, err)
	}
	defer file.Close()

	reader := csv.NewReader(file)
	reader.Comma = '\t' // Assuming tab-separated values

	records, err := reader.ReadAll()
	if err != nil {
		// EOF is expected if the file is empty or just created
		if err == io.EOF && len(records) == 0 {
			return nil
		}
		return fmt.Errorf("failed to read records from %s: %w", filePath, err)
	}

	for _, record := range records {
		if len(record) != 2 {
			fmt.Fprintf(os.Stderr, "Skipping malformed record in %s: expected 2 fields, got %d. Record: %v\n", filePath, len(record), record)
			continue
		}
		appName := record[0]
		timeStr := record[1]
		seconds, err := timeutils.ParseHHMMSSToSeconds(timeStr)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Skipping record with invalid time format in %s for app %s: %v. Error: %v\n", filePath, appName, timeStr, err)
			continue
		}
		activity[appName] = seconds
	}
	return nil
}

// writeDailyActivity writes the current activity data to the CSV file.
// filePath is the full path to the log file.
func writeDailyActivity(filePath string, activity map[string]int) error {
	// Ensure the directory exists
	dir := filepath.Dir(filePath)
	if _, err := os.Stat(dir); os.IsNotExist(err) {
		if err := os.MkdirAll(dir, 0755); err != nil { // MkdirAll also handles parent dirs like "daily_data"
			return fmt.Errorf("failed to create directory %s for log file: %w", dir, err)
		}
	}

	file, err := os.Create(filePath)
	if err != nil {
		return fmt.Errorf("failed to create/truncate daily log file %s: %w", filePath, err)
	}
	defer file.Close()

	writer := csv.NewWriter(file)
	writer.Comma = '\t' // Use tab as a separator

	for appName, totalSeconds := range activity {
		timeStr := timeutils.FormatSecondsToHHMMSS(totalSeconds)
		if err := writer.Write([]string{appName, timeStr}); err != nil {
			// Log the error but try to continue writing other entries
			fmt.Fprintf(os.Stderr, "Error writing record for app %s to %s: %v\n", appName, filePath, err)
		}
	}
	writer.Flush()
	if err := writer.Error(); err != nil {
		return fmt.Errorf("failed to flush data to %s: %w", filePath, err)
	}
	return nil
}


// StartLogging begins the process of monitoring active windows and logging activity.
// baseCacheDir should be like "/path/to/user/.cache/Watcher"
func StartLogging(baseCacheDir string) error {
	if baseCacheDir == "" {
		// Fallback to default if not provided, or handle error. For robustness, let's try a default.
		usr, err := user.Current()
		if err != nil {
			return fmt.Errorf("cannot determine user's home directory for default cache path: %w", err)
		}
		baseCacheDir = filepath.Join(usr.HomeDir, ".cache", "Watcher")
		fmt.Printf("[INFO] baseCacheDir not provided, defaulting to: %s\n", baseCacheDir)
	}


	// Ensure the base "daily_data" directory exists. writeDailyActivity will also ensure it.
	// This explicit check is good for clarity and early failure if needed.
	dailyDataRoot := filepath.Join(baseCacheDir, "daily_data")
	if _, err := os.Stat(dailyDataRoot); os.IsNotExist(err) {
		if err := os.MkdirAll(dailyDataRoot, 0755); err != nil {
			return fmt.Errorf("failed to create base daily_data directory %s: %w", dailyDataRoot, err)
		}
	}


	currentDate := time.Now().Format("2006-01-02")
	logFilePath, err := getDailyLogFilePath(baseCacheDir, currentDate)
	if err != nil {
		return fmt.Errorf("failed to determine log file path: %w", err)
	}

	dailyActivity := make(map[string]int)
	if err := loadDailyActivity(logFilePath, dailyActivity); err != nil { // loadDailyActivity takes full path
		fmt.Fprintf(os.Stderr, "Warning: Failed to load existing daily activity from %s: %v. Starting fresh for the day.\n", logFilePath, err)
		// Reset map just in case loadDailyActivity partially filled it before erroring
		dailyActivity = make(map[string]int)
	}

	ticker := time.NewTicker(1 * time.Second)
	defer ticker.Stop()

	// Periodic save interval (e.g., every 10 seconds)
	saveInterval := 10 * time.Second
	lastSaveTime := time.Now()

	fmt.Printf("Starting activity logging for %s. Log file: %s\n", currentDate, logFilePath)

	for {
		select {
		case <-ticker.C:
			// Check for date change
			now := time.Now()
			newDate := now.Format("2006-01-02")
			if newDate != currentDate {
				fmt.Printf("Date changed from %s to %s. Finalizing old log and starting new one.\n", currentDate, newDate)
				// Finalize writing for the old date
				if err := writeDailyActivity(logFilePath, dailyActivity); err != nil {
					fmt.Fprintf(os.Stderr, "Error writing final activity to %s: %v\n", logFilePath, err)
				}

				// Reset for the new date
				currentDate = newDate
				newLogFilePath, err := getDailyLogFilePath(baseCacheDir, currentDate)
				if err != nil {
					// This is a critical error, perhaps stop logging or retry
					return fmt.Errorf("CRITICAL: Failed to determine new log file path for %s: %w. Stopping.", currentDate, err)
				}
				logFilePath = newLogFilePath
				dailyActivity = make(map[string]int) // Reset activity map
				// Attempt to load data for the new day, in case the program was restarted on this new day
				if err := loadDailyActivity(logFilePath, dailyActivity); err != nil { // loadDailyActivity takes full path
					fmt.Fprintf(os.Stderr, "Warning: Failed to load existing daily activity from new log file %s: %v. Starting fresh.\n", logFilePath, err)
					dailyActivity = make(map[string]int)
				}
				lastSaveTime = now // Reset save timer
				fmt.Printf("Now logging for %s. New log file: %s\n", currentDate, logFilePath)
			}

			// Check for AFK status before logging activity
			userIsAFK, afkErr := afk.IsUserAFK(afkTimeoutMillis)
			if afkErr != nil {
				// Log the error but treat as not AFK to be safe (i.e., continue logging)
				fmt.Fprintf(os.Stderr, "Error checking AFK status: %v. Assuming user is active.\n", afkErr)
			}

			if userIsAFK {
				// If user is AFK, do not log activity for the current window.
				// Optionally, log AFK time to a special category or just skip.
				// For now, we just print a message if verbose, and skip logging.
				// fmt.Println("User is AFK. Skipping activity logging for this tick.")
			} else {
				// User is not AFK, proceed with logging activity.
				activeApp, err := getActiveWindowClassName()
				if err != nil {
					fmt.Fprintf(os.Stderr, "Error getting active window: %v\n", err)
					// Optionally, log to a special "Error" or "Unknown" category in dailyActivity
				} else {
					dailyActivity[activeApp]++ // Increment seconds for this app
				}
			}

			// Periodically save data
			if time.Since(lastSaveTime) >= saveInterval {
				//fmt.Printf("Saving activity to %s...\n", logFilePath) // Verbose
				if err := writeDailyActivity(logFilePath, dailyActivity); err != nil {
					fmt.Fprintf(os.Stderr, "Error writing activity to %s: %v\n", logFilePath, err)
				}
				lastSaveTime = time.Now()
			}
		}
	}
	// Unreachable in current infinite loop structure, but good for completeness
	// return nil
}
