package cmd

import (
	"database/sql"
	"fmt"
	"strings"
	"time"

	"github.com/BurntSushi/xgb"
	"github.com/BurntSushi/xgb/xproto"
	_ "github.com/mattn/go-sqlite3"
	"github.com/spf13/cobra"
)

type DB struct {
	connection *sql.DB
}

func (d *DB) InitConnection() error {
	db, err := sql.Open("sqlite3", "./watcher.db")
	if err != nil {
		return err
	}

	_, err = db.Exec(`CREATE TABLE IF NOT EXISTS daily_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    window_name TEXT,
    date TEXT,
    usage INTEGER
  )`)

	if err != nil {
		return err
	}

	d.connection = db
	return nil
}

func (d *DB) CloseConnection() {
	if d.connection != nil {
		d.connection.Close()
	}
}

func (d *DB) GetAppUsage(appName string, date string) (time.Duration, error) {
	var usage time.Duration
	err := d.connection.QueryRow(`SELECT usage FROM daily_logs WHERE window_name = ? AND date = ?`, appName, date).Scan(&usage)
	if err != nil {
		return 0, err
	}
	return usage, nil
}

func (d *DB) InsertOrUpdateAppUsage(appName string, date string, usage time.Duration) error {
	// first check if the app usage is already present in the db
	_, err := d.connection.Exec(`INSERT OR REPLACE INTO daily_logs (window_name, date, usage) VALUES (?, ?, ?)`, appName, date, usage)
	if err != nil {
		return err
	}
	return nil
}

// Display could be macos display or linux display
// activeWindow, err := Display.getActiveWindow()
// Display is an interface that has getActiveWindow method so that implementation of this method is abstracted away according to OS

// TODO: not used this interface as of now cause currently no plans to implement this for macos until I got one macos machine
type Display interface {
	GetActiveWindow() (*string, error)
}

type XDisplay struct {
	X     *xgb.Conn
	Usage map[string]time.Duration
}

func (d *XDisplay) GetActiveWindow() (*string, error) {
	aw, err := xproto.GetInputFocus(d.X).Reply()
	if err != nil {
		return nil, err
	}

	awClass, err := xproto.GetProperty(d.X, false, aw.Focus, xproto.AtomWmClass, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return nil, err
	}

	awTitle, err := xproto.GetProperty(d.X, false, aw.Focus, xproto.AtomWmName, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return nil, err
	}

	var appName string

	appNameParts := strings.Split(string(awClass.Value), "\x00")
	if len(appNameParts) > 1 {
		appName = appNameParts[1]
	} else {
		appName = "Unknown"
	}

	activeWindowTitle := string(awTitle.Value)

	terminals := []string{
		"Kitty", "Alacritty", "Terminator", "Tilda", "Guake", "Yakuake", "Roxterm", "Eterm", "Rxvt", "Xterm", "Tilix",
		"Lxterminal", "Konsole", "St", "Gnome-terminal", "Xfce4-terminal", "Terminology", "Extraterm",
	}

	if stringInSlice(appName, terminals) {
		if strings.Contains(activeWindowTitle, "Nvim") {
			appName = "NeoVim"
		} else if strings.Contains(activeWindowTitle, "Vim") {
			appName = "Vim"
		} else if strings.Contains(activeWindowTitle, "NVIM") {
			appName = "LunarVim"
		}
	}

	return &appName, nil
}

func (d *XDisplay) initXConnection() error {
	var err error
	d.X, err = xgb.NewConn()
	if err != nil {
		return err
	}

	setup := xproto.Setup(d.X)
	screen := setup.DefaultScreen(d.X)
	root := screen.Root

	err = xproto.ChangeWindowAttributesChecked(d.X, root, xproto.CwEventMask, []uint32{xproto.EventMaskPropertyChange}).Check()
	if err != nil {
		return err
	}

	return nil
}

func (d *XDisplay) closeXConnection() {
	if d.X != nil {
		d.X.Close()
	}
}

func stringInSlice(str string, slice []string) bool {
	for _, s := range slice {
		if str == s {
			return true
		}
	}
	return false
}

var watchCommand = &cobra.Command{
	Use:   "watch",
	Short: "It starts the watching your screen and takes the logs of your usage",
	Run: func(cmd *cobra.Command, args []string) {
		// logic to start taking logs and just printing the current app usages as time passes no string in db or logfiles

		// TODO: currently this logic is written in synchronous way, we need to convert this to go routines
		// implementation with GoRoutines will something like this, first to watch over the change in the current active window and other to check the AFK status of user every status

		display := XDisplay{
			X:     nil,
			Usage: make(map[string]time.Duration),
		}

		db := DB{
			connection: nil,
		}

		err := db.InitConnection()
		if err != nil {
			fmt.Println(err)
			return
		}

		display.initXConnection()

		date := time.Now().Format("2006-01-02")

		startTime := time.Now()
		activeWindow, err := display.GetActiveWindow()
		if err != nil {
			fmt.Println(err)
		}

		//for {
		//	if time.Now().Format("2006-01-02") != date {
		//		// TODO: insert or update the recent app usage in the db before emptying the usage map
		//		display.Usage[*activeWindow] += time.Since(startTime)
		//		db.InsertOrUpdateAppUsage(*activeWindow, date, display.Usage[*activeWindow])

		//    // delete all the key, value pair from display.Usage map
		//    for k := range display.Usage {
		//      delete(display.Usage, k)
		//    }
		//    fmt.Println(display.Usage)
		//		date = time.Now().Format("2006-01-02")
		//	}

		//	newWindow, err := display.GetActiveWindow()

		//	if err != nil {
		//		fmt.Println(err)
		//		continue
		//	}

		//	// Sleep for 100ms
		//	time.Sleep(100 * time.Millisecond)

		//	if *newWindow != *activeWindow {
		//		display.Usage[*activeWindow] += time.Since(startTime)
		//		fmt.Println(display.Usage)
		//		db.InsertOrUpdateAppUsage(*activeWindow, date, display.Usage[*activeWindow])
		//		activeWindow = newWindow
		//		startTime = time.Now()
		//		runtime.GC()
		//	}
		//}

		//		for {
		//			newWindow, err := display.GetActiveWindow()
		//			if err != nil {
		//				fmt.Println(err)
		//				continue
		//			}
		//
		//			for *activeWindow == *newWindow {
		//				time.Sleep(100 * time.Millisecond)
		//				newWindow, err = display.GetActiveWindow()
		//				if err != nil {
		//					fmt.Println(err)
		//				}
		//			}
		//
		//			display.Usage[*activeWindow] += time.Since(startTime)
		//			fmt.Println(display.Usage)
		//			db.InsertOrUpdateAppUsage(*activeWindow, date, display.Usage[*activeWindow])
		//			activeWindow = newWindow
		//			startTime = time.Now()
		//		}

		for {
			event, xerr := display.X.WaitForEvent()
			if xerr != nil {
				fmt.Println(xerr)
				continue
			}

			switch event.(type) {
			case xproto.PropertyNotifyEvent:
        fmt.Println("FocusInEvent")
				newWindow, err := display.GetActiveWindow()
				if err != nil {
					fmt.Println(err)
					continue
				}
				if activeWindow != nil {
					display.Usage[*activeWindow] += time.Since(startTime)
					fmt.Println(display.Usage)
					db.InsertOrUpdateAppUsage(*activeWindow, date, display.Usage[*activeWindow])
				}
				activeWindow = newWindow
				startTime = time.Now()
			}
		}
	},
}

func init() {
	rootCmd.AddCommand(watchCommand)
}
