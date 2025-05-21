package reporter

import (
	"encoding/csv"
	"fmt"
	"io"
	"os"
	"os/user"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
	"watcher/pkg/timeutils" // Import for time utilities
)

// ANSI Color Codes
const (
	ColorGrey      = "\033[90m"
	ColorBlue      = "\033[94m"
	ColorGreen     = "\033[92m"
	ColorYellow    = "\033[93m"
	ColorRed       = "\033[91m"
	ColorPurple    = "\033[95m"
	ColorDarkCyan  = "\033[36m"
	ColorBold      = "\033[1m"
	ColorUnderline = "\033[4m"
	ColorReset     = "\033[0m"
)

// Helper function to apply color and reset
func colorize(text string, color string) string {
	return color + text + ColorReset
}

// Helper function to apply bold and color
func colorizeBold(text string, color string) string {
	return ColorBold + color + text + ColorReset + ColorReset // Double reset for safety with bold
}

// Specific color functions
func Grey(text string) string      { return colorize(text, ColorGrey) }
func Blue(text string) string      { return colorize(text, ColorBlue) }
func Green(text string) string     { return colorize(text, ColorGreen) }
func Yellow(text string) string    { return colorize(text, ColorYellow) }
func Red(text string) string       { return colorize(text, ColorRed) }
func Purple(text string) string    { return colorize(text, ColorPurple) }
func DarkCyan(text string) string  { return colorize(text, ColorDarkCyan) }
func Bold(text string) string      { return ColorBold + text + ColorReset }
func Underline(text string) string { return ColorUnderline + text + ColorReset }

// getDailyLogFilePath constructs the path for the daily log file.
// baseCacheDir should be like "/path/to/user/.cache/Watcher"
func getDailyLogFilePath(baseCacheDir string, dateStr string) (string, error) {
	if baseCacheDir == "" {
		return "", fmt.Errorf("baseCacheDir cannot be empty")
	}
	dailyDataDir := filepath.Join(baseCacheDir, "daily_data")
	return filepath.Join(dailyDataDir, dateStr+".csv"), nil
}

// loadDailyLog reads the CSV file for the given date and parses it into a map.
// baseCacheDir is the root directory for Watcher's cache (e.g., /home/user/.cache/Watcher)
func loadDailyLog(baseCacheDir string, date string) (map[string]int, error) {
	filePath, err := getDailyLogFilePath(baseCacheDir, date)
	if err != nil {
		return nil, fmt.Errorf("failed to construct log file path for date %s using base %s: %w", date, baseCacheDir, err)
	}
	activityData := make(map[string]int)
	file, err := os.Open(filePath)
	if err != nil {
		if os.IsNotExist(err) {
			// Return a distinct error or a message that can be specifically checked for "no data"
			return nil, fmt.Errorf("no data found for %s (file %s does not exist)", date, filePath)
		}
		return nil, fmt.Errorf("failed to open daily log file %s: %w", filePath, err)
	}
	defer file.Close()

	reader := csv.NewReader(file)
	reader.Comma = '\t'
	records, err := reader.ReadAll()
	if err != nil {
		if err == io.EOF && len(records) == 0 { // Handle empty file
			return activityData, nil
		}
		return nil, fmt.Errorf("failed to read records from %s: %w", filePath, err)
	}

	for i, record := range records {
		if len(record) != 2 {
			fmt.Fprintf(os.Stderr, "Skipping malformed record in %s (line %d): expected 2 fields, got %d. Record: %v\n", filePath, i+1, len(record), record)
			continue
		}
		appName := record[0]
		timeStr := record[1]
		seconds, err := timeutils.ParseHHMMSSToSeconds(timeStr)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Skipping record with invalid time format in %s (line %d) for app %s: %v. Error: %v\n", filePath, i+1, appName, timeStr, err)
			continue
		}
		activityData[appName] = seconds
	}
	return activityData, nil
}

// AppActivity holds the application name and its total active seconds.
type AppActivity struct {
	AppName string
	Seconds int
}

// sortActivityData sorts the activity data by seconds in descending order.
func sortActivityData(activityData map[string]int) []AppActivity {
	var sortedActivities []AppActivity
	for appName, seconds := range activityData {
		sortedActivities = append(sortedActivities, AppActivity{appName, seconds})
	}
	sort.Slice(sortedActivities, func(i, j int) bool {
		return sortedActivities[i].Seconds > sortedActivities[j].Seconds
	})
	return sortedActivities
}

// getCurrentDateString returns the current date in "YYYY-MM-DD" format.
func getCurrentDateString() string {
	return time.Now().Format("2006-01-02")
}

// --- Date Logic for Week ---
func getWeekNumberAndYear(date time.Time) (int, int) {
	return date.ISOWeek()
}

func getDatesOfWeek(weekNum int, year int) ([]string, error) {
	if weekNum < 1 || weekNum > 53 {
		return nil, fmt.Errorf("invalid week number: %d, must be between 1 and 53", weekNum)
	}
	// Jan 4th is always in week 1 of its year
	startOfYear := time.Date(year, time.January, 4, 0, 0, 0, 0, time.UTC)
	// Find Monday of week 1
	for startOfYear.Weekday() != time.Monday {
		startOfYear = startOfYear.AddDate(0, 0, -1)
	}
	// Monday of the target week
	mondayOfWeek := startOfYear.AddDate(0, 0, (weekNum-1)*7)
	
	// Crucial check: ensure the calculated Monday actually belongs to the requested ISO week and year
	actualYear, actualWeek := mondayOfWeek.ISOWeek()
	if actualYear != year || actualWeek != weekNum {
		return nil, fmt.Errorf("calculated Monday (%s) is in week %d, year %d, not week %d, year %d as requested. This often means the requested week does not exist or is ambiguous for the year.", mondayOfWeek.Format("2006-01-02"), actualWeek, actualYear, weekNum, year)
	}

	var dates []string
	for i := 0; i < 7; i++ {
		dates = append(dates, mondayOfWeek.AddDate(0, 0, i).Format("2006-01-02"))
	}
	return dates, nil
}

func getCurrentWeekString() string {
	year, weekNum := getWeekNumberAndYear(time.Now())
	return fmt.Sprintf("W%02d-%d", weekNum, year)
}

func parseWeekString(weekString string) (int, int, string, error) {
	if weekString == "" || weekString == "current" {
		year, weekNum := getWeekNumberAndYear(time.Now())
		return weekNum, year, fmt.Sprintf("W%02d-%d", weekNum, year), nil
	}
	if !strings.HasPrefix(weekString, "W") {
		return 0, 0, "", fmt.Errorf("invalid week string format: expected Www-YYYY, got %s", weekString)
	}
	parts := strings.Split(strings.TrimPrefix(weekString, "W"), "-")
	if len(parts) != 2 {
		return 0, 0, "", fmt.Errorf("invalid week string format: expected Www-YYYY, got %s", weekString)
	}
	weekNum, err := strconv.Atoi(parts[0])
	if err != nil {
		return 0, 0, "", fmt.Errorf("invalid week number in %s: %w", weekString, err)
	}
	year, err := strconv.Atoi(parts[1])
	if err != nil {
		return 0, 0, "", fmt.Errorf("invalid year in %s: %w", weekString, err)
	}
	if weekNum < 1 || weekNum > 53 { // Basic check, getDatesOfWeek will do a more precise validation
		return 0, 0, "", fmt.Errorf("week number %d out of range (1-53)", weekNum)
	}
	return weekNum, year, fmt.Sprintf("W%02d-%d", weekNum, year), nil
}
// --- End Date Logic for Week ---

// getWeeklyActivity aggregates activity data for all days in a specified week.
// baseCacheDir is the root directory for Watcher's cache.
func getWeeklyActivity(baseCacheDir string, weekString string) (map[string]int, map[string]int, string, error) {
	weekNum, year, canonicalWeekString, err := parseWeekString(weekString)
	if err != nil {
		return nil, nil, "", fmt.Errorf("failed to parse week string '%s': %w", weekString, err)
	}
	datesInWeek, err := getDatesOfWeek(weekNum, year)
	if err != nil {
		return nil, nil, "", fmt.Errorf("failed to get dates for week W%02d-%d: %w", weekNum, year, err)
	}
	weeklyAppUsage := make(map[string]int)
	dailyTotals := make(map[string]int)
	var daysWithDataFound int
	for _, dateStr := range datesInWeek {
		dailyData, errLoad := loadDailyLog(baseCacheDir, dateStr) // Pass baseCacheDir
		if errLoad != nil {
			if strings.Contains(errLoad.Error(), "does not exist") {
				dailyTotals[dateStr] = 0 // Ensure day is in map even if no data
				continue
			}
			// For other errors, log it but try to continue with other days
			fmt.Fprintf(os.Stderr, "Warning: Error loading data for %s: %v\n", dateStr, errLoad)
			dailyTotals[dateStr] = 0 // Still record the day as having 0 time due to error
			continue
		}
		daysWithDataFound++
		dayTotalSeconds := 0
		for appName, seconds := range dailyData {
			weeklyAppUsage[appName] += seconds
			dayTotalSeconds += seconds
		}
		dailyTotals[dateStr] = dayTotalSeconds
	}
	if daysWithDataFound == 0 {
		return nil, nil, canonicalWeekString, fmt.Errorf("no activity data found for any day in week %s", canonicalWeekString)
	}
	return weeklyAppUsage, dailyTotals, canonicalWeekString, nil
}

// getWeeklyCacheFilePath constructs the path for the weekly cache file.
// baseCacheDir is the root directory for Watcher's cache.
// weekString is already canonical Www-YYYY from parseWeekString.
func getWeeklyCacheFilePath(baseCacheDir string, weekString string) (string, error) {
	if baseCacheDir == "" {
		return "", fmt.Errorf("baseCacheDir cannot be empty")
	}
	analysisDir := filepath.Join(baseCacheDir, "Analysis")
	if err := os.MkdirAll(analysisDir, 0755); err != nil { // Ensure directory exists
		return "", fmt.Errorf("failed to create analysis directory %s: %w", analysisDir, err)
	}
	return filepath.Join(analysisDir, weekString+".csv"), nil
}

// writeWeeklyCache saves the aggregated weekly data to a CSV file.
// baseCacheDir is the root directory for Watcher's cache.
func writeWeeklyCache(baseCacheDir string, weekString string, weeklyAppUsage map[string]int, dailyTotals map[string]int, datesInWeek []string) error {
	filePath, err := getWeeklyCacheFilePath(baseCacheDir, weekString)
	if err != nil {
		return fmt.Errorf("failed to get weekly cache file path for %s using base %s: %w", weekString, baseCacheDir, err)
	}
	file, err := os.Create(filePath)
	if err != nil {
		return fmt.Errorf("failed to create weekly cache file %s: %w", filePath, err)
	}
	defer file.Close()

	writer := csv.NewWriter(file)
	writer.Comma = '\t'
	dayShortNames := []string{"Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"}
	for i, dateStr := range datesInWeek {
		if i >= len(dayShortNames) { break } // Should not happen
		dayName := dayShortNames[i]
		totalSeconds, _ := dailyTotals[dateStr] // Default to 0 if day somehow missing
		timeFormatted := timeutils.FormatSecondsToHHMMSS(totalSeconds)
		if err := writer.Write([]string{dayName, timeFormatted}); err != nil {
			return fmt.Errorf("failed to write daily total for %s (%s) to %s: %w", dayName, dateStr, filePath, err)
		}
	}
	sortedAppActivities := sortActivityData(weeklyAppUsage)
	for _, activity := range sortedAppActivities {
		appName := activity.AppName
		timeFormatted := timeutils.FormatSecondsToHHMMSS(activity.Seconds)
		if err := writer.Write([]string{timeFormatted, appName}); err != nil {
			return fmt.Errorf("failed to write app usage for %s to %s: %w", appName, filePath, err)
		}
	}
	writer.Flush()
	if errW := writer.Error(); errW != nil {
		return fmt.Errorf("writer error after flush for %s: %w", filePath, errW)
	}
	return nil
}

// ShowDailySummary loads, processes, and prints the activity summary for a given date.
// baseCacheDir is the root directory for Watcher's cache.
func ShowDailySummary(baseCacheDir string, dateString string) error {
	if baseCacheDir == "" {
		return fmt.Errorf("baseCacheDir must be provided to ShowDailySummary")
	}
	if dateString == "" || dateString == "today" {
		dateString = getCurrentDateString()
	}
	activityData, err := loadDailyLog(baseCacheDir, dateString)
	if err != nil {
		if strings.Contains(err.Error(), "does not exist") {
			fmt.Printf("No data found for %s.\n", dateString)
			return nil 
		}
		return fmt.Errorf("failed to load daily log for %s using base %s: %w", dateString, baseCacheDir, err)
	}
	if len(activityData) == 0 {
		fmt.Printf("No activity recorded for %s.\n", dateString)
		return nil
	}
	sortedActivities := sortActivityData(activityData)
	totalSeconds := 0
	for _, activity := range sortedActivities {
		totalSeconds += activity.Seconds
	}
	totalScreenTimeFormatted := timeutils.FormatSecondsToHHMMSS(totalSeconds)
	titleDate := dateString
	if dateString == getCurrentDateString() {
		titleDate = "Today"
	}
	fmt.Printf("\n%s %s\n", Underline(Bold(titleDate+"'s Screen-Time:")), Green(Bold(totalScreenTimeFormatted)))
	fmt.Println(Bold(Blue("----------------------------------")))
	fmt.Println(Bold(Purple("     App Usages:")))
	for _, activity := range sortedActivities {
		appName := activity.AppName
		if appName == "" || appName == "Unknown/Desktop" {
			appName = "Unknown/Desktop/AFK"
		}
		timeFormatted := timeutils.FormatSecondsToHHMMSS(activity.Seconds)
		fmt.Printf("  %s %s\n", DarkCyan(appName+":"), Yellow(timeFormatted))
	}
	fmt.Println(Bold(Blue("----------------------------------")))
	return nil
}

// ShowWeeklySummary loads, processes, caches, and prints the activity summary for a given week.
// baseCacheDir is the root directory for Watcher's cache.
func ShowWeeklySummary(baseCacheDir string, weekInputString string) error {
	if baseCacheDir == "" {
		return fmt.Errorf("baseCacheDir must be provided to ShowWeeklySummary")
	}
	weeklyAppUsage, dailyTotals, canonicalWeekString, err := getWeeklyActivity(baseCacheDir, weekInputString)
	if err != nil {
		if strings.Contains(err.Error(), "no activity data found for any day in week") {
			fmt.Printf("No activity data found for week %s.\n", weekInputString)
			return nil
		}
		if strings.Contains(err.Error(), "does not exist or is ambiguous for the year") || strings.Contains(err.Error(), "not found in year"){
			fmt.Printf("Week %s could not be determined or does not exist for the specified year.\n", weekInputString)
			return nil
		}
		return fmt.Errorf("failed to get weekly activity for '%s' using base %s: %w", weekInputString, baseCacheDir, err)
	}
	parsedWeekNum, parsedYear, _, err := parseWeekString(weekInputString) // Re-parse to get components
	if err != nil { 
		return fmt.Errorf("internal error: failed to re-parse week string '%s' for ShowWeeklySummary: %w", weekInputString, err)
	}
	datesInWeek, err := getDatesOfWeek(parsedWeekNum, parsedYear)
	if err != nil { 
		return fmt.Errorf("internal error: failed to get dates for week W%02d-%d for ShowWeeklySummary: %w", parsedWeekNum, parsedYear, err)
	}
	err = writeWeeklyCache(baseCacheDir, canonicalWeekString, weeklyAppUsage, dailyTotals, datesInWeek)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Warning: Failed to write weekly cache for %s using base %s: %v\n", canonicalWeekString, baseCacheDir, err)
	}
	totalWeekSeconds := 0
	for _, seconds := range dailyTotals {
		totalWeekSeconds += seconds
	}
	totalWeekScreenTimeFormatted := Green(Bold(timeutils.FormatSecondsToHHMMSS(totalWeekSeconds)))
	titleWeek := canonicalWeekString
	if canonicalWeekString == getCurrentWeekString() {
		titleWeek = "This Week's"
	} else {
		titleWeek = "Week " + canonicalWeekString
	}
	fmt.Printf("\n%s %s\n", Underline(Bold(titleWeek+" Screen-Time:")), totalWeekScreenTimeFormatted)
	fmt.Println(Bold(Blue("----------------------------------")))
	dayShortNames := []string{"Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"}
	for i, dateStr := range datesInWeek {
		if i >= len(dayShortNames) { break }
		dayName := dayShortNames[i]
		secondsForDay, _ := dailyTotals[dateStr]
		dayColor := DarkCyan
		if dateStr == getCurrentDateString() {
			dayColor = Purple
		}
		fmt.Printf("  %s %s: %s\n", dayColor(dayName), Grey("("+dateStr[5:]+")"), Yellow(timeutils.FormatSecondsToHHMMSS(secondsForDay)))
	}
	fmt.Println(Bold(Blue("----------------------------------")))
	fmt.Println(Bold(Purple("     App Usages (Weekly Total):")))
	sortedWeeklyActivities := sortActivityData(weeklyAppUsage)
	if len(sortedWeeklyActivities) == 0 {
		fmt.Println(Grey("  No application usage recorded for the week."))
	} else {
		for _, activity := range sortedWeeklyActivities {
			appName := activity.AppName
			if appName == "" || appName == "Unknown/Desktop" {
				appName = "Unknown/Desktop/AFK"
			}
			timeFormatted := timeutils.FormatSecondsToHHMMSS(activity.Seconds)
			fmt.Printf("    %s %s\n", DarkCyan(appName+":"), Yellow(timeFormatted))
		}
	}
	fmt.Println(Bold(Blue("----------------------------------")))
	return nil
}

```
