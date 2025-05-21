package cmd

import (
	"fmt"
	"os"
	"path/filepath" // Ensure filepath is imported
	"watcher/pkg/watcher"

	"github.com/spf13/cobra"
)

// startCmd represents the start command
var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Starts the Watcher logging service",
	Long: `Starts monitoring application usage and logs screen time.
This command runs continuously until manually stopped (e.g., via Ctrl+C).`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Starting Watcher logging service...")
		
		baseCacheDir, err := GetBaseCacheDir()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error determining base cache directory: %v\n", err)
			os.Exit(1)
		}

		// Call the StartLogging function from your watcher package
		if err := watcher.StartLogging(baseCacheDir); err != nil {
			fmt.Fprintf(os.Stderr, "Error during watcher logging: %v\n", err)
			os.Exit(1)
		}
		fmt.Println("Watcher logging service stopped.") // This line might only be reached if StartLogging can exit gracefully
	},
}

// GetBaseCacheDir determines the base directory for Watcher's cache.
// It defaults to user's cache directory + /Watcher.
func GetBaseCacheDir() (string, error) {
	userCacheDir, err := os.UserCacheDir()
	if err != nil {
		return "", fmt.Errorf("failed to get user cache directory: %w", err)
	}
	return filepath.Join(userCacheDir, "Watcher"), nil
}

func init() {
	rootCmd.AddCommand(startCmd)

	// Here you can define flags and configuration settings for the start command.
	// Example:
	// startCmd.Flags().BoolP("daemon", "d", false, "Run in daemon mode (background)")
}
