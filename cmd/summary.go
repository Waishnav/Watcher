package cmd

import (
	"fmt"
	"os"
	"regexp"
	"path/filepath" // Ensure filepath is imported
	"watcher/pkg/reporter"

	"github.com/spf13/cobra"
)

// summaryCmd represents the summary command
var summaryCmd = &cobra.Command{
	Use:   "summary [today|week|YYYY-MM-DD|Www-YYYY]",
	Short: "Displays screen time summaries",
	Long: `Displays screen time summaries. Defaults to today if no argument is provided.
Can show summaries for 'today', 'week', a specific date (YYYY-MM-DD), or a specific week (Www-YYYY).`,
	Args: cobra.MaximumNArgs(1), // Allows zero or one argument
	Run: func(cmd *cobra.Command, args []string) {
		baseCacheDir, err := GetBaseCacheDir() // Same helper as in start.go, or define locally/share
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error determining base cache directory: %v\n", err)
			os.Exit(1)
		}

		target := "today" // Default target
		if len(args) > 0 {
			target = args[0]
		}

		// Regex for YYYY-MM-DD
		datePattern := `^\d{4}-\d{2}-\d{2}$`
		isDate, _ := regexp.MatchString(datePattern, target)

		// Regex for Www-YYYY (e.g., W40-2023 or W01-2024)
		// Allows for 1 or 2 digits for week number: Ww-YYYY or Www-YYYY
		weekPattern := `^W\d{1,2}-\d{4}$`
		isWeek, _ := regexp.MatchString(weekPattern, target)

		var err error
		if target == "today" {
			fmt.Println("Displaying summary for today...")
			err = reporter.ShowDailySummary(baseCacheDir, "today")
		} else if target == "week" {
			fmt.Println("Displaying summary for the current week...")
			err = reporter.ShowWeeklySummary(baseCacheDir, "current")
		} else if isDate {
			fmt.Printf("Displaying summary for date: %s...\n", target)
			err = reporter.ShowDailySummary(baseCacheDir, target)
		} else if isWeek {
			fmt.Printf("Displaying summary for week: %s...\n", target)
			err = reporter.ShowWeeklySummary(baseCacheDir, target)
		} else {
			fmt.Fprintf(os.Stderr, "Error: Invalid argument '%s'. Please use 'today', 'week', a date in YYYY-MM-DD format, or a week in Www-YYYY format.\n", target)
			os.Exit(1)
		}

		if err != nil {
			fmt.Fprintf(os.Stderr, "Error displaying summary: %v\n", err)
			// Check if the error is due to "no data found" to provide a cleaner exit for that case.
			// reporter.ShowDailySummary and reporter.ShowWeeklySummary already print "No data found"
			// and return nil for that specific case, so this error handling here will
			// catch other unexpected errors.
			// If those functions returned a specific error type for "no data", we could check it here.
			// For now, a generic error message is fine.
			os.Exit(1)
		}
	},
}

func init() {
	rootCmd.AddCommand(summaryCmd)

	// Here you can define flags and configuration settings for the summary command.
}

// GetBaseCacheDir (duplicated from start.go for now, consider moving to a shared cmd utility)
func GetBaseCacheDir() (string, error) {
	userCacheDir, err := os.UserCacheDir()
	if err != nil {
		return "", fmt.Errorf("failed to get user cache directory: %w", err)
	}
	return filepath.Join(userCacheDir, "Watcher"), nil
}
