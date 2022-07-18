import os
import time

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

def is_afk(timeout):
    timeout = timeout * 60 * 1000 - 200 # minimizing 200 milisec error
    #If the AFK feature is installed
    time_since_last_input = int(os.popen("xprintidle").read()[:-1])
    if (time_since_last_input > timeout):
        video_playback = os.popen("""pacmd list-sink-inputs  | grep -w state | grep -i 'RUNNING'""").read()
        # if playback is not running as well as user is AFK
        if "RUNNING" in video_playback:
            return False
        # if playback is running is background as well as user is AFK
        else:
            return True
    return False

# testing out
if __name__ == "__main__":
    while True:
        time.sleep(1)
        print(is_afk(0.05))
