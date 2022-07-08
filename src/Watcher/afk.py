import os
def IsAFK():
    time_since_last_input = int(os.popen("xprintidle").read())
    if time_since_last_input >= 300000: # 3min no input == AFK
        video_playback = os.popen("""pacmd list-sink-inputs  | grep -w state | grep -i 'CORKED'""").read()
        # if playback is not running as well as user is AFK
        if "CORKED" in video_playback:
            return True
        # if playback is running is background as well as user is AFK
        else:
            return False
    else:
        return False

