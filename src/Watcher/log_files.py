import os
import csv
import time
import get_windows as x

# get current time whenever the function is called
def get_time():
    t = os.popen('''date +"%T"''').read()
    return t[0:-1]

# get date of today
def get_date():
    d = os.popen('''date +"%Y-%m-%d"''').read()
    return d[0:-1]

def append_line_in_csv(date, opened_time, window_name):
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/raw_data/"+date+".csv"
    Data = [opened_time, window_name]
    with open(filename, 'a') as csvfile:
        csvwriter = csv.writer(csvfile, delimiter='\t')
        csvwriter.writerow(Data)

def is_date_changed(a):
    result = False
    while not(result):
        time.sleep(1)
        b = get_date()
        if a != b :
            result = True
        else:
            result = False
    return result

# Expected Behaviour == if date got changed then append line in new csv file after initializing the csv file
# also if usr is AFK then append line

# TODO: AFK feature devlopement (it will be developed after completing alpha product (after whole project up end running)

afk = False
def log_creation():
    global afk

    while True:
        previous_window = x.active_window()
        if x.is_window_changed(previous_window) and not(afk):
            next_window = x.active_window()
            opened_at = get_time()
            date = get_date()
            append_line_in_csv(date, opened_at, next_window)

        if afk:
            opened_at = get_time()
            append_line_in_csv(date, opened_at, "AFK")


if __name__ == "__main__":
    log_creation()
