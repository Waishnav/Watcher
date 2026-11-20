import ast
import os
import shutil
import subprocess

BUS_NAME = "org.micro_watcher.Focus"
OBJECT_PATH = "/org/micro_watcher/Focus"
INTERFACE = "org.micro_watcher.Focus"

TERMINALS = [
    "Kitty", "Alacritty", "Terminator", "Tilda", "Guake", "Yakuake",
    "Roxterm", "Eterm", "Rxvt", "Xterm", "Tilix", "Lxterminal",
    "Konsole", "St", "Gnome-terminal", "Xfce4-terminal", "Terminology",
    "Extraterm", "Mate-terminal", "Console", "Ptyxis", "Blackbox",
]


def _run(args, timeout=2):
    try:
        return subprocess.run(
            args,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None


def _is_wayland():
    return (
        os.environ.get("XDG_SESSION_TYPE", "").lower() == "wayland"
        or bool(os.environ.get("WAYLAND_DISPLAY"))
    )


def _is_gnome():
    return "GNOME" in os.environ.get("XDG_CURRENT_DESKTOP", "").upper()


def _gnome_dbus_call(method):
    if shutil.which("gdbus") is None:
        return None

    result = _run(
        [
            "gdbus", "call",
            "--session",
            "--dest", BUS_NAME,
            "--object-path", OBJECT_PATH,
            "--method", f"{INTERFACE}.{method}",
        ]
    )
    if result is None or result.returncode != 0:
        return None

    try:
        return ast.literal_eval(result.stdout.strip())
    except (ValueError, SyntaxError):
        return None


def backend_status():
    """Return (available, human-readable backend description)."""
    if _is_wayland():
        if not _is_gnome():
            return False, (
                "Wayland session detected, but only GNOME/Wayland is supported "
                f"in this build (desktop: {os.environ.get('XDG_CURRENT_DESKTOP', 'unknown')})."
            )
        if shutil.which("gdbus") is None:
            return False, "Wayland session detected, but gdbus is not installed."

        version = _gnome_dbus_call("GetVersion")
        if version is None:
            return False, (
                "GNOME focus extension is not loaded. Enable 'micro-watcher Focus' "
                "in the Extensions app, or run the installer and log out/back in."
            )
        return True, f"GNOME Wayland (micro-watcher Focus v{version[0] if version else '?'})"

    missing = [
        tool for tool in ("xdotool", "xprop", "xprintidle")
        if shutil.which(tool) is None
    ]
    if missing:
        return False, "X11 tools missing: " + ", ".join(missing)
    return True, "X11 (xdotool/xprop/xprintidle)"


def _display_name(app_id, wm_class):
    candidate = wm_class or app_id or ""
    if "." in candidate:
        candidate = candidate.rsplit(".", 1)[-1]
    if candidate.endswith("-server"):
        candidate = candidate[:-len("-server")]
    return candidate.capitalize()


def _terminal_override(name, title):
    if name not in TERMINALS:
        return name
    if "nvim" in title:
        return "NeoVim"
    if "vim" in title:
        return "Vim"
    return name


def _active_window_wayland():
    values = _gnome_dbus_call("GetActiveWindow")
    if not isinstance(values, (tuple, list)) or len(values) < 3:
        return ""

    app_id, wm_class, title = (str(value or "") for value in values[:3])
    if not app_id and not wm_class:
        return ""
    return _terminal_override(_display_name(app_id, wm_class), title)


# get title name of app that user working on (X11)
def active_window_title():
    actv_id = os.popen("xdotool getwindowfocus").read()[:-1]
    if not actv_id:
        return ""
    title = os.popen(f'xprop -id {actv_id} WM_NAME').read()
    title = title[19:-2]
    a = title.find('"')
    title = title[a + 1:] if a >= 0 else title
    if "XGetWindowProperty[_NET_ACTIVE_WINDOW] failed" in title:
        title = ""
    if "\n" in title:
        title = "Unknown"
    return title


# get classname of app that user working on (X11)
def active_window_x11():
    actv_id = os.popen("xdotool getwindowfocus").read()[:-1]
    if len(actv_id) == 4:
        active_window = ""
    else:
        active_window = os.popen(f"xprop -id {actv_id} | grep CLASS ").read()
        if active_window != "":
            active_window = active_window[19:-1].replace('"', "").split(", ")[1]

    if "XGetWindowProperty[_NET_ACTIVE_WINDOW] failed" in active_window:
        active_window = ""
    if "\n" in active_window:
        active_window = "Unknown"

    active_window = active_window.capitalize()
    return _terminal_override(active_window, active_window_title())


def active_window():
    if _is_wayland():
        return _active_window_wayland()
    return active_window_x11()


# returns true if user has move to next app which is not the same as previous
def previous_window(array_of_window, active_window):
    array_of_window.remove(active_window)
    array_of_window.append(active_window)
    return array_of_window[-2]


if __name__ == "__main__":
    print(active_window())
    print(backend_status()[1])
