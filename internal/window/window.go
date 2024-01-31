package window

import (
	"strings"

	"github.com/BurntSushi/xgb"
	"github.com/BurntSushi/xgb/xproto"
)

func GetActiveWindowTitle() (string, error) {
	X, err := xgb.NewConn()
	if err != nil {
		return "", err
	}
	// postponded the close command until whole function is executed
	defer X.Close()

	// get the root window
	// root := xgb.Setup(X).DefaultScreen(X).Root

	activeWindow, err := xproto.GetInputFocus(X).Reply()
	if err != nil {
		return "", err
	}

	// get the class of the active window
	awTitle, err := xproto.GetProperty(X, false, activeWindow.Focus, xproto.AtomWmName, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return "", err
	}

	activeWindowTitle := string(awTitle.Value)

	return activeWindowTitle, nil

}

func GetActiveWindowAppname() (string, error) {
	X, err := xgb.NewConn()
	if err != nil {
		return "", err
	}
	// postponded the close command until whole function is executed
	defer X.Close()

	// get the root window
	// root := xgb.Setup(X).DefaultScreen(X).Root

	activeWindow, err := xproto.GetInputFocus(X).Reply()
	if err != nil {
		return "", err
	}

	// get the class of the active window
	class, err := xproto.GetProperty(X, false, activeWindow.Focus, xproto.AtomWmClass, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
	if err != nil {
		return "", err
	}

	// convert the class to string
	classString := string(class.Value)
	appName := strings.Split(classString, "\x00")[1]

	awTitle, err := xproto.GetProperty(X, false, activeWindow.Focus, xproto.AtomWmName, xproto.GetPropertyTypeAny, 0, (1<<32)-1).Reply()
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
