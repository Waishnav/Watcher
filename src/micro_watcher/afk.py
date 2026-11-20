import os
import re
import shutil
import subprocess
import time

IDLE_BUS = "org.gnome.Mutter.IdleMonitor"
IDLE_PATH = "/org/gnome/Mutter/IdleMonitor/Core"
IDLE_INTERFACE = "org.gnome.Mutter.IdleMonitor"


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


def idle_seconds():
    """Return seconds since the last input, or None if it cannot be measured."""
    if _is_wayland():
        if shutil.which("gdbus") is None:
            return None
        result = _run(
            [
                "gdbus", "call",
                "--session",
                "--dest", IDLE_BUS,
                "--object-path", IDLE_PATH,
                "--method", f"{IDLE_INTERFACE}.GetIdletime",
            ]
        )
        if result is None or result.returncode != 0:
            return None
        match = re.search(r"uint64\s+([0-9]+)", result.stdout)
        if match is None:
            return None
        return int(match.group(1)) / 1000.0

    if shutil.which("xprintidle") is None:
        return None
    result = _run(["xprintidle"])
    if result is None or result.returncode != 0:
        return None
    try:
        return int(result.stdout.strip()) / 1000.0
    except ValueError:
        return None


def media_playing():
    """Return True when a PulseAudio/PipeWire sink input is running."""
    if shutil.which("pactl") is None:
        return False
    result = _run(["pactl", "list", "sink-inputs"])
    if result is None or result.returncode != 0:
        return False
    return "State: RUNNING" in result.stdout


def is_afk(timeout):
    idle = idle_seconds()
    if idle is None:
        # Do not silently pause tracking when the idle source is unavailable;
        # the caller reports the backend problem separately.
        return False
    if idle > timeout * 60 - 0.1:
        return not media_playing()
    return False


# checks if currently in afk mode, only returns true once
def get_afk_status(afk_active, timeout):
    if (is_afk(timeout)):
        return True
    elif returned_from_afk(afk_active, timeout):
        return True
    else:
        return False


def returned_from_afk(afk_active, timeout):
    has_returned = (afk_active and not (is_afk(timeout)))
    return has_returned


# testing out
if __name__ == "__main__":
    while True:
        time.sleep(1)
        print(is_afk(0.05))
