import os

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
    return array_of_window[-2]

if __name__ == "__main__":
    print(active_window())
