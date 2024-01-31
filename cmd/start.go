/*
Copyright © 2023 Waishnav <waishnav.work@gmail.com>
*/
package cmd

import (
	"fmt"
	"watcher-go/internal/afkutils"
	db_utils "watcher-go/internal/db"
	"watcher-go/internal/timeutils"
	"watcher-go/internal/window"

	"github.com/spf13/cobra"
)

// startCmd represents the start command
var startCmd = &cobra.Command{
	Use:   "start",
	Short: "It starts the watcher to taking the logs",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("start called")
		// we need to start the watcher logging service here
		db, err := db_utils.Init()
		if err != nil {
			fmt.Println("Error initializing database")
		}
		defer db.Close()

		afkTimeout := 1
		// importing todays data instead of initializing it
		data := make(map[string]int)

		for {

			// date := timeutil.GetDate()
			afk := afkutils.IsAFK(afkTimeout)

			if !afk {
				activeAppName, err := window.GetActiveWindowAppname()
				if err != nil {
					fmt.Println("Error getting active window app name")
				}
				// get previous usage of active app
				previousUsage := data[activeAppName]
				// suspend execution for 1 second
				timeutils.Sleep(1)

				if afkutils.IsAFK(afkTimeout) {
					// if user is afk, then we need to update the data with the previous usage
					userAFKTime := afkutils.GetTimeSinceLastInput() / 1000
					data["AFK"] += int(userAFKTime)
					continue
				}
				// if user is not afk, then we need to update the data with the previous usage + 1
				data[activeAppName] = previousUsage + 1
				// fmt.Println(data)
				// db query to update the data of the active app of the day
				db_utils.UpdateData(db, activeAppName, data[activeAppName], timeutils.GetDate())
				db_utils.GetTodaysData(db, timeutils.GetDate())
			}
		}
	},
}

func init() {
	rootCmd.AddCommand(startCmd)
}
