package cmd

import (
	"fmt"
	"runtime"
	"strings"
	"time"

	"github.com/BurntSushi/xgb"
	"github.com/BurntSushi/xgb/xproto"
	"github.com/spf13/cobra"
)

var watchCommand = &cobra.Command{
	Use:   "watch",
	Short: "It starts the watching your screen and takes the logs of your usage",
	Run: func(cmd *cobra.Command, args []string) {
		// logic to start taking logs and just printing the current app usages as time passes no string in db or logfiles

		// TODO: currently this logic is written in synchronous way, we need to convert this to go routines
		// implementation with GoRoutines will something like this, first to watch over the change in the current active window and other to check the AFK status of user every status
		defer closeXConnection()

		err := initXConnection()
		if err != nil {
			fmt.Println("Error initializing X connection:", err)
			return
		}

		startTime := time.Now()
		activeWindow, err := ActiveWindow()
		if err != nil {
			fmt.Println(err)
		}

		for {
			newWindow, err := ActiveWindow()
			if err != nil {
				fmt.Println(err)
				continue
			}

			// Sleep for 200ms
			time.Sleep(200 * time.Millisecond)

			if newWindow.WindowName != activeWindow.WindowName {
				activeWindow.Usage = time.Since(startTime)
				fmt.Println(*activeWindow)
				activeWindow = newWindow
				startTime = time.Now()
				runtime.GC()
			}
		}
	},
}

func init() {
	rootCmd.AddCommand(watchCommand)
}

var X *xgb.Conn

// var aw *xproto.GetInputFocusReply
// var awClass *xproto.GetPropertyReply

type Window struct {
	WindowName  string
	WindowTitle string
	Usage       time.Duration
}

func ActiveWindow() (*Window, error) {
	aw, err := xproto.GetInputFocus(X).Reply()
	if err != nil {
		return nil, err
	}

	awClass, err := xproto.GetProperty(X, false, aw.Focus, xproto.AtomWmClass, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return nil, err
	}

	awTitle, err := xproto.GetProperty(X, false, aw.Focus, xproto.AtomWmName, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return nil, err
	}

	appName := strings.Split(string(awClass.Value), "\x00")[1]
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

	return &Window{
		WindowName:  appName,
		WindowTitle: activeWindowTitle,
	}, nil
}

func initXConnection() error {
	var err error
	X, err = xgb.NewConn()
	if err != nil {
		return err
	}
	return nil
}

func closeXConnection() {
	if X != nil {
		X.Close()
	}
}

// func GetActiveWindowAppname(aw *xproto.GetInputFocusReply, awClass *xproto.GetPropertyReply) (string, error) {
func GetActiveWindow() (string, error) {
	aw, err := xproto.GetInputFocus(X).Reply()
	if err != nil {
		return "", err
	}

	// get the class of the active window
	awClass, err := xproto.GetProperty(X, false, aw.Focus, xproto.AtomWmClass, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return "", err
	}

	// convert the class to string
	appName := strings.Split(string(awClass.Value), "\x00")[1] // splitting depending on null byte

	awTitle, err := xproto.GetProperty(X, false, aw.Focus, xproto.AtomWmName, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return "", err
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

	return appName, nil
}

func stringInSlice(str string, slice []string) bool {
	for _, s := range slice {
		if str == s {
			return true
		}
	}
	return false
}
