import os
import time
from afk import get_afk_status

# get title name of app that user working on
def active_window_title():
    active_window_title = os.popen('''xprop -id $(xdotool getwindowfocus) WM_NAME''').read()[19:-2]
    a = active_window_title.find('"')
    active_window_title = active_window_title[a+1:]
    if "XGetWindowProperty[_NET_ACTIVE_WINDOW] failed" in active_window_title:
        active_window_title = ""
    if "\n" in active_window_title:
        active_window_title = "Unknown"
    return active_window_title

# get classname of app that user working on
def active_window():
    actv_id = os.popen("xdotool getwindowfocus").read()[:-1]
    if len(actv_id) == 4:
        active_window = ""
    else:
        active_window = os.popen("xprop -id $(xdotool getwindowfocus) | grep CLASS ").read()
        if active_window != "":
            active_window = active_window[19:-1].replace('''"''', "").split(", ")[1]

    if "XGetWindowProperty[_NET_ACTIVE_WINDOW] failed" in active_window:
        active_window = ""
    if "\n" in active_window:
        active_window = "Unknown"
    # check whether user is using nvim or vim
    active_window = active_window.capitalize()
    aw_title = active_window_title()
    terminals = ["Kitty", "Alacritty", "Terminator", "Tilda", "Guake", "Yakuake", "Roxterm", "Eterm", "Rxvt", "Xterm", "Tilix", "Lxterminal", "Konsole", "St", "Gnome-terminal", "Xfce4-terminal", "Terminology", "Extraterm"]
    if active_window in terminals:
        try:
            if "nvim" in aw_title:
                active_window = "NeoVim"
            elif "vim" in aw_title:
                active_window = "Vim"
        except TypeError:
            None
    return active_window

# returns true if user has move to next app which is not the same as previous
def previous_window(array_of_window, active_window):
    array_of_window.remove(active_window)
    array_of_window.append(active_window)
    return l[-2]


### what to do after window get change I've to append one line in csv data file in following format
### opened-time      closed-time      time-spent     window_class_name      window_title_name

### and whenever the user puts particular command it will make report till the time for that day and shows that report in terminal
if __name__ == "__main__":
    while True:
        time.sleep(1)
        print(active_window_title())
        print(os.popen('''xdotool getwindowfocus getwindowname''').read())
