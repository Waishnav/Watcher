/*
Copyright © 2024 Waishnav <waishnav.work@gmail.com>
*/

package cmd

import (
	"fmt"

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
		fmt.Println(err)
	}
}

func Init() {
	rootCmd.AddCommand(versionCmd)
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print the version number of Watcher",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Watcher v2.0")
	},
}
