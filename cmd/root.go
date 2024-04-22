/*
Copyright © 2024 Waishnav <waishnav.work@gmail.com>
*/

package cmd

import (
	"os"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "Watcher",
	Short: "Watcher - Minimal open source screen-time tracker",
	Long: `Watcher tracks your screen time and app usage across the day. 
It provides summaries for daily and weekly usage.`,
}

func Execute() {
	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}
