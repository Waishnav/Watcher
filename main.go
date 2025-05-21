package main

import (
	"fmt"
	"os"
	"watcher/cmd"
)

func main() {
	// The initCacheDirs function was previously here.
	// It should be called by commands that require it, e.g., 'start' or 'summary' if they read/write cache.
	// For now, let's assume the 'start' command will ensure cache dirs exist before logging,
	// and 'summary' commands will handle missing files gracefully (as they mostly do).
	// If a global setup is needed before any command runs, it could be part of rootCmd's PersistentPreRun.
	// For this refactor, we'll remove it from main.go and expect individual commands or their packages
	// to handle their prerequisites. The `watcher.StartLogging` already ensures directory for its logs,
	// and `reporter.ShowDailySummary/ShowWeeklySummary` handle missing files by reporting "no data".
	// `reporter.writeWeeklyCache` also ensures its directory.
	// So, direct call to `initCacheDirs()` from main is no longer strictly needed with Cobra structure.

	cmd.Execute()
}
