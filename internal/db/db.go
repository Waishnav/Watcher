// all db manipulation should and queries should be done here

package db_utils

import (
	"database/sql"
	"fmt"

	_ "github.com/mattn/go-sqlite3"
)

func Init() (*sql.DB, error) {
	// create db if not exists
	// create tables if not exists

	db, err := sql.Open("sqlite3", "./watcher_db.db")
	if err != nil {
		panic(err)
	}
	// defer db.Close()

	// create table if not exists
	_, err = db.Exec("CREATE TABLE IF NOT EXISTS daily_usage (id INTEGER PRIMARY KEY AUTOINCREMENT, app_name TEXT, time_spent INTEGER, date TEXT)")
	if err != nil {
		panic(err)
	}

	return db, err
}

func InsertData(db *sql.DB, appName string, timeSpent int, date string) {
	// insert data into table
	_, err := db.Exec("INSERT INTO daily_usage (date, app_name, time_spent) VALUES (?, ?, ?)", date, appName, timeSpent)
	if err != nil {
		panic(err)
	}
}

func UpdateData(db *sql.DB, appName string, timeSpent int, date string) {
	// insert or replace data into table
	_, err := db.Exec("INSERT OR REPLACE INTO daily_usage (date, app_name, time_spent) VALUES (?, ?, ?)", date, appName, timeSpent)

	if err != nil {
		panic(err)
	}
}

func GetData(db *sql.DB, appName string, date string) int {
	// get data from table
	var timeSpent int
	err := db.QueryRow("SELECT time_spent FROM daily_usage WHERE app_name = ? AND date = ?", appName, date).Scan(&timeSpent)
	if err != nil {
		panic(err)
	}

	return timeSpent
}

func GetTodaysData(db *sql.DB, date string) map[string]int {
	// get data from table
	rows, err := db.Query("SELECT app_name, time_spent FROM daily_usage WHERE date = ?", date)
	if err != nil {
		panic(err)
	}
	defer rows.Close()

	data := make(map[string]int)

	for rows.Next() {
		var appName string
		var timeSpent int
		err = rows.Scan(&appName, &timeSpent)
		if err != nil {
			panic(err)
		}
		data[appName] = timeSpent
	}
	fmt.Println(data)

	return data
}
