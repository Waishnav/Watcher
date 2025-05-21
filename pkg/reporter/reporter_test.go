package reporter

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"watcher/pkg/timeutils" // For direct use in tests if needed, or rely on reporter's usage
)

// Helper function to create a temporary directory for testing.
// Returns the path to the temp directory and a cleanup function.
func setupTestCacheDir(t *testing.T) (string, func()) {
	tmpDir, err := os.MkdirTemp("", "watcher_test_cache_")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	
	// Create subdirectories like the application would use
	if err := os.MkdirAll(filepath.Join(tmpDir, "daily_data"), 0755); err != nil {
		os.RemoveAll(tmpDir) // Attempt cleanup on error
		t.Fatalf("Failed to create daily_data subdir: %v", err)
	}
	if err := os.MkdirAll(filepath.Join(tmpDir, "Analysis"), 0755); err != nil {
		os.RemoveAll(tmpDir) // Attempt cleanup on error
		t.Fatalf("Failed to create Analysis subdir: %v", err)
	}

	cleanup := func() {
		os.RemoveAll(tmpDir)
	}
	return tmpDir, cleanup
}

// Helper function to create a dummy daily log file.
func createDummyDailyLog(t *testing.T, baseCacheDir string, date string, content string) string {
	logPath, err := getDailyLogFilePath(baseCacheDir, date) // Uses the actual function to ensure path logic is also implicitly tested
	if err != nil {
		t.Fatalf("Failed to get daily log file path for %s: %v", date, err)
	}

	// Ensure directory for the log file exists (getDailyLogFilePath doesn't create it)
	logDir := filepath.Dir(logPath)
	if err := os.MkdirAll(logDir, 0755); err != nil {
		t.Fatalf("Failed to create directory for dummy log %s: %v", logDir, err)
	}

	err = os.WriteFile(logPath, []byte(content), 0644)
	if err != nil {
		t.Fatalf("Failed to write dummy daily log to %s: %v", logPath, err)
	}
	return logPath
}


// --- Tests for loadDailyLog ---
func TestLoadDailyLog_ValidFile(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	date := "2023-01-01"
	logContent := "Firefox\t01:00:00\nTerminal\t00:30:15\n"
	createDummyDailyLog(t, baseCacheDir, date, logContent)

	expected := map[string]int{
		"Firefox":  3600,
		"Terminal": 1815,
	}

	loadedData, err := loadDailyLog(baseCacheDir, date)
	if err != nil {
		t.Fatalf("loadDailyLog failed for valid file: %v", err)
	}
	if !reflect.DeepEqual(loadedData, expected) {
		t.Errorf("loadDailyLog returned %v, want %v", loadedData, expected)
	}
}

func TestLoadDailyLog_NonExistentFile(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	date := "2023-01-02" // No file created for this date

	_, err := loadDailyLog(baseCacheDir, date)
	if err == nil {
		t.Fatalf("loadDailyLog expected an error for non-existent file, got nil")
	}
	// Check if the error message indicates file not found (as per current loadDailyLog implementation)
	if !strings.Contains(err.Error(), "does not exist") {
		t.Errorf("loadDailyLog error message %q does not indicate file not found", err.Error())
	}
}

func TestLoadDailyLog_MalformedTime(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	date := "2023-01-03"
	// One valid entry, one malformed time, one with wrong number of fields
	logContent := "Chrome\t00:10:00\nVSCode\tinvalid:time\nNotes\t00:05:00\tUnexpectedField\nSpotify\t00:20:00"
	createDummyDailyLog(t, baseCacheDir, date, logContent)

	expected := map[string]int{
		"Chrome":  600,
		"Spotify": 1200, // VSCode and Notes should be skipped
	}
	// Note: loadDailyLog currently logs to os.Stderr for malformed lines.
	// For testing, we might want to capture stderr or pass a logger.
	// For now, we just check the returned map.

	loadedData, err := loadDailyLog(baseCacheDir, date)
	if err != nil {
		t.Fatalf("loadDailyLog failed for file with malformed time: %v", err)
	}
	if !reflect.DeepEqual(loadedData, expected) {
		t.Errorf("loadDailyLog with malformed time returned %v, want %v", loadedData, expected)
	}
}

func TestLoadDailyLog_EmptyFile(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	date := "2023-01-04"
	logContent := "" // Empty file
	createDummyDailyLog(t, baseCacheDir, date, logContent)

	expected := map[string]int{} // Expect an empty map

	loadedData, err := loadDailyLog(baseCacheDir, date)
	if err != nil {
		t.Fatalf("loadDailyLog failed for empty file: %v", err)
	}
	if len(loadedData) != 0 { // Check length first for empty map
		t.Errorf("loadDailyLog for empty file returned %v, want empty map", loadedData)
	}
	if !reflect.DeepEqual(loadedData, expected) { // For completeness
		t.Errorf("loadDailyLog for empty file returned %v, want %v", loadedData, expected)
	}
}


// --- Test for sortActivityData ---
func TestSortActivityData(t *testing.T) {
	input := map[string]int{
		"AppC": 100,
		"AppA": 300,
		"AppB": 200,
	}
	expected := []AppActivity{
		{AppName: "AppA", Seconds: 300},
		{AppName: "AppB", Seconds: 200},
		{AppName: "AppC", Seconds: 100},
	}

	sorted := sortActivityData(input)

	if !reflect.DeepEqual(sorted, expected) {
		t.Errorf("sortActivityData returned:\n%v\nwant:\n%v", sorted, expected)
	}

	// Test with empty input
	inputEmpty := map[string]int{}
	expectedEmpty := []AppActivity{}
	sortedEmpty := sortActivityData(inputEmpty)
	if len(sortedEmpty) != 0 { // Check length for empty slice
		t.Errorf("sortActivityData with empty map returned %v, want empty slice", sortedEmpty)
	}
	if !reflect.DeepEqual(sortedEmpty, expectedEmpty) { // For completeness
		t.Errorf("sortActivityData with empty map returned %v, want %v", sortedEmpty, expectedEmpty)
	}
}


// --- Tests for Date Logic for Week ---
func TestGetDatesOfWeek(t *testing.T) {
	tests := []struct {
		name      string
		weekNum   int
		year      int
		expected  []string
		expectErr bool
		errSubstr string // Substring to check in error message
	}{
		{
			name:    "Regular Week Mid-Year",
			weekNum: 28, // Example: Week 28 of 2023
			year:    2023,
			expected: []string{
				"2023-07-10", "2023-07-11", "2023-07-12", "2023-07-13", "2023-07-14", "2023-07-15", "2023-07-16",
			}, // Monday to Sunday
			expectErr: false,
		},
		{
			name:    "Week 1 of Year",
			weekNum: 1,
			year:    2023, // Jan 1 2023 is Sunday. Week 1 starts Jan 2.
			expected: []string{
				"2023-01-02", "2023-01-03", "2023-01-04", "2023-01-05", "2023-01-06", "2023-01-07", "2023-01-08",
			},
			expectErr: false,
		},
		{
			name:    "Week 52 of Year (no week 53)",
			weekNum: 52,
			year:    2023, // 2023 ends on Sunday, Dec 31, part of week 52.
			expected: []string{
				"2023-12-25", "2023-12-26", "2023-12-27", "2023-12-28", "2023-12-29", "2023-12-30", "2023-12-31",
			},
			expectErr: false,
		},
		{
			name:      "Week 53 (non-existent for year)",
			weekNum:   53,
			year:      2023, // 2023 does not have a week 53
			expectErr: true,
			errSubstr: "not week 53, year 2023 as requested", // Error from check in getDatesOfWeek
		},
		{
            name:    "Year with Week 53 (e.g. 2020)",
            weekNum: 53,
            year:    2020, // 2020 had a week 53
            expected: []string{
                "2020-12-28", "2020-12-29", "2020-12-30", "2020-12-31", "2021-01-01", "2021-01-02", "2021-01-03",
            },
            expectErr: false,
        },
		{
            name:    "Week 1, Year where Jan 1 is a Friday (e.g. 2021)",
            weekNum: 1,
            year:    2021, // Jan 1 2021 is Friday. Week 1 of 2021 starts Jan 4.
            expected: []string{
                "2021-01-04", "2021-01-05", "2021-01-06", "2021-01-07", "2021-01-08", "2021-01-09", "2021-01-10",
            },
            expectErr: false,
        },
		{
			name:      "Invalid Week Number (too low)",
			weekNum:   0,
			year:      2023,
			expectErr: true,
			errSubstr: "invalid week number",
		},
		{
			name:      "Invalid Week Number (too high)",
			weekNum:   54,
			year:      2023,
			expectErr: true,
			errSubstr: "invalid week number",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getDatesOfWeek(tt.weekNum, tt.year)
			if (err != nil) != tt.expectErr {
				t.Errorf("getDatesOfWeek(%d, %d) error = %v, expectErr %v", tt.weekNum, tt.year, err, tt.expectErr)
				return
			}
			if tt.expectErr {
				if err != nil && tt.errSubstr != "" && !strings.Contains(err.Error(), tt.errSubstr) {
					t.Errorf("getDatesOfWeek(%d, %d) error %q does not contain expected substring %q", tt.weekNum, tt.year, err.Error(), tt.errSubstr)
				}
			} else {
				if !reflect.DeepEqual(got, tt.expected) {
					t.Errorf("getDatesOfWeek(%d, %d) = %v, want %v", tt.weekNum, tt.year, got, tt.expected)
				}
			}
		})
	}
}

// Minimal test for parseWeekString as it's indirectly tested via getWeeklyActivity/ShowWeeklySummary
// but good to have direct tests too.
func TestParseWeekString(t *testing.T) {
    // Assuming getCurrentWeekString and getWeekNumberAndYear are correct for "current" case.
    // More detailed tests for various Www-YYYY formats.
    _, _, currentCanon, _ := parseWeekString("current")
    if !strings.HasPrefix(currentCanon, "W") {
        t.Errorf("parseWeekString(\"current\") canonical form %s is invalid", currentCanon)
    }

    tests := []struct{
        input string
        wantWeek int
        wantYear int
        wantCanon string
        wantErr bool
    }{
        {"W28-2023", 28, 2023, "W28-2023", false},
        {"W1-2024", 1, 2024, "W01-2024", false}, // Check canonical form padding
        {"W52-2023", 52, 2023, "W52-2023", false},
        {"invalid", 0,0,"", true},
        {"W55-2023", 0,0,"", true}, // Invalid week num
        {"W10-badyear", 0,0,"", true},
    }

    for _, tc := range tests {
        t.Run(tc.input, func(t *testing.T){
            week, year, canon, err := parseWeekString(tc.input)
            if (err != nil) != tc.wantErr {
                t.Fatalf("parseWeekString(%q) error = %v, wantErr %v", tc.input, err, tc.wantErr)
            }
            if !tc.wantErr {
                if week != tc.wantWeek || year != tc.wantYear || canon != tc.wantCanon {
                    t.Errorf("parseWeekString(%q) = (%d, %d, %q), want (%d, %d, %q)", 
                        tc.input, week, year, canon, tc.wantWeek, tc.wantYear, tc.wantCanon)
                }
            }
        })
    }
}

// Note: Testing ShowDailySummary and ShowWeeklySummary directly is complex as they print to stdout
// and have many dependencies. They are better tested via integration tests or by refactoring them
// to return strings/data structures rather than printing directly.
// For this subtask, we focus on testable units like loadDailyLog, sortActivityData, and date logic.
// Testing getWeeklyActivity and writeWeeklyCache would follow similar patterns to loadDailyLog,
// involving setting up multiple daily log files and then verifying the aggregated map or the content of the cache file.

// --- Tests for getWeeklyActivity ---
func TestGetWeeklyActivity_ValidData(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	// Create dummy logs for a week
	// Week 28 of 2023: July 10 (Mon) to July 16 (Sun)
	createDummyDailyLog(t, baseCacheDir, "2023-07-10", "Firefox\t01:00:00\nTerminal\t00:30:00\n") // Mon: FF 3600, Term 1800
	createDummyDailyLog(t, baseCacheDir, "2023-07-11", "Firefox\t00:30:00\nVSCode\t01:00:00\n")   // Tue: FF 1800, VSCode 3600
	createDummyDailyLog(t, baseCacheDir, "2023-07-13", "Terminal\t01:00:00\n")                  // Thu: Term 3600 (Wed, Fri, Sat, Sun no data)
	
	weekStr := "W28-2023"
	expectedWeeklyAppUsage := map[string]int{
		"Firefox":  3600 + 1800,
		"Terminal": 1800 + 3600,
		"VSCode":   3600,
	}
	expectedDailyTotals := map[string]int{
		"2023-07-10": 3600 + 1800, // Mon
		"2023-07-11": 1800 + 3600, // Tue
		"2023-07-12": 0,           // Wed
		"2023-07-13": 3600,        // Thu
		"2023-07-14": 0,           // Fri
		"2023-07-15": 0,           // Sat
		"2023-07-16": 0,           // Sun
	}

	weeklyAppUsage, dailyTotals, canonicalWeek, err := getWeeklyActivity(baseCacheDir, weekStr)
	if err != nil {
		t.Fatalf("getWeeklyActivity(%q) failed: %v", weekStr, err)
	}
	if canonicalWeek != weekStr {
		t.Errorf("getWeeklyActivity canonicalWeek = %q, want %q", canonicalWeek, weekStr)
	}
	if !reflect.DeepEqual(weeklyAppUsage, expectedWeeklyAppUsage) {
		t.Errorf("getWeeklyActivity weeklyAppUsage = %v, want %v", weeklyAppUsage, expectedWeeklyAppUsage)
	}
	if !reflect.DeepEqual(dailyTotals, expectedDailyTotals) {
		t.Errorf("getWeeklyActivity dailyTotals = %v, want %v", dailyTotals, expectedDailyTotals)
	}
}

func TestGetWeeklyActivity_NoDataForWeek(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	weekStr := "W01-2024" // Assume no logs exist for this week in the temp dir

	_, _, _, err := getWeeklyActivity(baseCacheDir, weekStr)
	if err == nil {
		t.Fatalf("getWeeklyActivity(%q) expected error for no data, got nil", weekStr)
	}
	if !strings.Contains(err.Error(), "no activity data found") {
		t.Errorf("getWeeklyActivity(%q) error %q does not contain 'no activity data found'", weekStr, err.Error())
	}
}

func TestGetWeeklyActivity_InvalidWeekString(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	weekStr := "InvalidWeek-2023"
	_, _, _, err := getWeeklyActivity(baseCacheDir, weekStr)
	if err == nil {
		t.Fatalf("getWeeklyActivity(%q) expected error for invalid week string, got nil", weekStr)
	}
	if !strings.Contains(err.Error(), "invalid week string format") {
		t.Errorf("getWeeklyActivity(%q) error %q does not contain 'invalid week string format'", weekStr, err.Error())
	}
}


// --- Tests for writeWeeklyCache and getWeeklyCacheFilePath ---
func TestWriteAndGetWeeklyCache(t *testing.T) {
	baseCacheDir, cleanup := setupTestCacheDir(t)
	defer cleanup()

	weekStr := "W10-2023"
	weeklyAppUsage := map[string]int{"AppX": 7200, "AppY": 3600}
	// Dates for W10-2023: Mar 6 to Mar 12
	datesInWeek, err := getDatesOfWeek(10, 2023)
	if err != nil {
		t.Fatalf("Test setup: getDatesOfWeek failed: %v", err)
	}
	dailyTotals := map[string]int{
		"2023-03-06": 3600, // Mon
		"2023-03-07": 3600, // Tue
		"2023-03-08": 0,    // Wed
		"2023-03-09": 3600, // Thu (AppX: 3600, AppY:0) - This doesn't fully align with weeklyAppUsage, but tests file writing format
		"2023-03-10": 0,    // Fri
		"2023-03-11": 0,    // Sat
		"2023-03-12": 0,    // Sun
	}
	
	// Modify dailyTotals to be more consistent with weeklyAppUsage for a better test
	// Let AppX be 2hr on Mon, AppY be 1hr on Tue.
	dailyTotals = map[string]int{
        datesInWeek[0]: 7200, // Mon: AppX
        datesInWeek[1]: 3600, // Tue: AppY
        datesInWeek[2]: 0,
        datesInWeek[3]: 0,
        datesInWeek[4]: 0,
        datesInWeek[5]: 0,
        datesInWeek[6]: 0,
    }


	err = writeWeeklyCache(baseCacheDir, weekStr, weeklyAppUsage, dailyTotals, datesInWeek)
	if err != nil {
		t.Fatalf("writeWeeklyCache failed: %v", err)
	}

	// Verify file content
	cacheFilePath, err := getWeeklyCacheFilePath(baseCacheDir, weekStr)
	if err != nil {
		t.Fatalf("getWeeklyCacheFilePath failed: %v", err)
	}

	if _, err := os.Stat(cacheFilePath); os.IsNotExist(err) {
		t.Fatalf("writeWeeklyCache did not create the file at %s", cacheFilePath)
	}

	content, err := os.ReadFile(cacheFilePath)
	if err != nil {
		t.Fatalf("Failed to read weekly cache file %s: %v", cacheFilePath, err)
	}

	// Expected content based on the data written:
	// Mon	02:00:00  (7200s from dailyTotals for datesInWeek[0])
	// Tue	01:00:00  (3600s from dailyTotals for datesInWeek[1])
	// ... other days 00:00:00
	// 02:00:00	AppX (7200s from weeklyAppUsage)
	// 01:00:00	AppY (3600s from weeklyAppUsage)
	// Note: sortActivityData sorts weeklyAppUsage, so AppX (7200) comes before AppY (3600)

	expectedLines := []string{
		"Mon\t" + timeutils.FormatSecondsToHHMMSS(dailyTotals[datesInWeek[0]]),
		"Tue\t" + timeutils.FormatSecondsToHHMMSS(dailyTotals[datesInWeek[1]]),
		"Wed\t" + timeutils.FormatSecondsToHHMMSS(0),
		"Thu\t" + timeutils.FormatSecondsToHHMMSS(0),
		"Fri\t" + timeutils.FormatSecondsToHHMMSS(0),
		"Sat\t" + timeutils.FormatSecondsToHHMMSS(0),
		"Sun\t" + timeutils.FormatSecondsToHHMMSS(0),
		timeutils.FormatSecondsToHHMMSS(7200) + "\tAppX",
		timeutils.FormatSecondsToHHMMSS(3600) + "\tAppY",
	}
	
	actualContentStr := strings.TrimSpace(string(content)) // Trim trailing newline if any
	actualLines := strings.Split(actualContentStr, "\n")

	if len(actualLines) != len(expectedLines) {
		t.Errorf("Weekly cache file has %d lines, want %d. Content:\n%s", len(actualLines), len(expectedLines), actualContentStr)
	}

	for i, expectedLine := range expectedLines {
		if i < len(actualLines) {
			if strings.TrimSpace(actualLines[i]) != expectedLine {
				t.Errorf("Weekly cache line %d = %q, want %q", i+1, strings.TrimSpace(actualLines[i]), expectedLine)
			}
		} else {
			t.Errorf("Weekly cache missing line %d: want %q", i+1, expectedLine)
		}
	}
}

```
