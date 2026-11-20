import os
import csv
import time
from datetime import datetime
import micro_watcher.get_windows as x
import micro_watcher.afk as y
from micro_watcher.time_operations import time_difference, time_addition, convert

# get current time whenever the function is called
def get_time():
    return datetime.now().strftime("%H:%M:%S")

# get date of today
def get_date():
    return datetime.now().strftime("%Y-%m-%d")

def update_csv(date, Data):
    filename = os.environ['HOME']+"/.cache/Watcher/daily_data/"+date+".csv"
    overwrite_Data = []
    with open(filename, 'w') as csvfile:
        for x,y in Data.items():
            overwrite_Data.append([y,x])

        csvwriter = csv.writer(csvfile, delimiter='\t')
        csvwriter.writerows(overwrite_Data)

# Expected Behaviour == if date got changed then append line in new csv file after initializing the csv file
# also if usr is AFK then append line
def import_data(file):
    with open(file, 'r') as f:
         raw_data = f.readlines()
    data = dict()
    #l = []
    for x in raw_data:
        x = x.split('\t')
        a = {x[1][:-1]:x[0]}
        #l.append(x[1][:-1])
        data.update(a)
    return data


# TODO: AFK feature devlopement (it will be developed after completing alpha product (after whole project up end running)

def log_creation():
    data_root = os.environ['HOME'] + "/.cache/Watcher"
    os.makedirs(data_root + "/daily_data", exist_ok=True)
    os.makedirs(data_root + "/Analysis", exist_ok=True)

    available, status = x.backend_status()
    if not available:
        print("micro-watcher: cannot start tracking: " + status)
        return False
    print("Tracking via " + status + " (press Ctrl+C to stop)")

    filename = os.environ['HOME']+"/.cache/Watcher/daily_data/"+get_date()+".csv"
    if not(os.path.isfile(filename)):
        creat_file = os.environ['HOME']+"/.cache/Watcher/daily_data/"+get_date()+".csv"
        with open(creat_file, 'w') as fp:
            pass

    afk = False
    afkTimeout = 1 # timeout in minutes
    data = import_data(filename)
    while True:
        date = get_date()
        filename = os.environ['HOME']+"/.cache/Watcher/daily_data/"+date+".csv"
        afk = y.is_afk(afkTimeout)

        if not(afk):
            active_window = x.active_window()
            usage = data.get(active_window)
            if usage == None:
                usage = "00:00:00"

            time.sleep(1)
            if y.is_afk(afkTimeout):
                idle = y.idle_seconds()
                if idle is not None:
                    usage = time_difference(convert(int(round(idle))), usage)

            usage = time_addition("00:00:01", usage)
            data.update({active_window : usage})
            if os.path.isfile(os.environ['HOME']+"/.cache/Watcher/daily_data/"+get_date()+".csv"):
                update_csv(get_date(), data)
            elif not(os.path.isfile(os.environ['HOME']+"/.cache/Watcher/daily_data/"+get_date()+".csv")):
                new_filename = os.environ['HOME']+"/.cache/Watcher/daily_data/"+get_date()+".csv"
                with open(new_filename, 'w') as fp:
                    pass

                data.clear()
        else:
            # Do not busy-loop while idle; the next iteration checks whether
            # the user has returned or media is still playing.
            time.sleep(1)


if __name__ == "__main__":
    log_creation()

