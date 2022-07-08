import os
import time
from ewmh import EWMH

class window:
    def __init__(self, class_name, title_name):
        self.class_name = class_name
        self.title_name = title_name

# get title name of app that user working on
def active_window_title():
    try:
        win = EWMH().getActiveWindow()
        active_window_title = win.get_wm_name()
    except AttributeError:
        active_window_title = "unknown"
    active_window_title = active_window_title.capitalize()
    return active_window_title

# get classname of app that user working on
def active_window():
    try:
        win = EWMH().getActiveWindow()
        active_window = win.get_wm_class()[1]
    except AttributeError:
        active_window = "unknown"

    if len(active_window) > 20:
        active_window = "unknown"
    elif "\n" in active_window:
        active_window = "unknown"
    active_window = active_window.capitalize()

    # check whether user is using nvim or vim
    aw_title = active_window_title()
    terminals = ["Kitty", "Alacritty", "Terminator", "Tilda", "Guake", "Yakuake", "Roxterm", "Eterm", "Rxvt", "Xterm", "Tilix", "Lxterminal", "Konsole", "St", "Gnome-terminal", "Xfce4-terminal", "Terminology", "Extraterm"]
    if active_window in terminals:
        if "Nvim" in aw_title:
            active_window = "NeoVim"
        elif "Vim" in aw_title:
            active_window = "Vim"
    return active_window

# returns true if user has move to next app which is not the same as previous
def is_window_changed(a):
    result = False
    time.sleep(0.1)
    b = active_window()
    if a != b :
        result = True
    return result

### what to do after window get change I've to append one line in csv data file in following format
### opened-time      closed-time      time-spent     window_class_name      window_title_name

### and whenever the user puts particular command it will make report till the time for that day and shows that report in terminal
