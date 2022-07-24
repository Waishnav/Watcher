import os
import csv
import time
import get_windows as x
import afk as y
from time_operations import time_difference

# get current time whenever the function is called
def get_time():
    t = os.popen('''date +"%T"''').read()
    return t[0:-1]

# get date of today
def get_date():
    d = os.popen('''date +"%Y-%m-%d"''').read()
    return d[0:-1]

def append_line_in_csv(date, opened_time, closed_time, window_name):
    user = os.getlogin()
    time_spent = time_difference(opened_time, closed_time)
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/raw_data/"+date+".csv"
    Data = [closed_time, time_spent,  window_name]
    with open(filename, 'a') as csvfile:
        csvwriter = csv.writer(csvfile, delimiter='\t')
        csvwriter.writerow(Data)

# Expected Behaviour == if date got changed then append line in new csv file after initializing the csv file
# also if usr is AFK then append line

# TODO: AFK feature devlopement (it will be developed after completing alpha product (after whole project up end running)

def log_creation():
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/raw_data/"+get_date()+".csv"
    if not(os.path.isfile(filename)):
        with open(filename, 'a') as csvfile:
            csvwriter = csv.writer(csvfile, delimiter='\t')
            csvwriter.writerow([get_time(), "00:00:00", ""])
    logout_time = os.popen("tail -n1 " + filename).read().split("\t")[0]
    append_line_in_csv(get_date(), logout_time, get_time(), "User-logged-in")

    afk = False
    afkTimeout = 3 # timeout in minutes

    while True:
        opened_at = get_time()
        previous_window = x.active_window()

        if (y.returned_from_afk(afk, afkTimeout)):
            previous_window = "AFK"
            if os.path.exists(filename):
                opened_at = os.popen(" tail -n1 "+ filename).read().split("\t")[0]
            else:
                previous_date = os.popen("""date -d "1 day ago" '+%Y-%m-%d'""").read()[:-1]
                opened_at = os.popen(" tail -n1 ~/.cache/Watcher/raw_data/"+previous_date+".csv").read().split("\t")[0]
            afk = False

        if (x.is_window_changed(previous_window, afk, afkTimeout) and not afk):
            if(y.is_afk(afkTimeout)):
                afk = True
                # minimizing error
                closed_at = time_difference("00:02:58", get_time())
            else:
                closed_at = get_time() # for next_window its the opening time

            date = get_date()
            filename = "/home/"+os.getlogin()+"/.cache/Watcher/raw_data/"+date+".csv"
            append_line_in_csv(date, opened_at, closed_at, previous_window)

if __name__ == "__main__":
    #log_creation()
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/raw_data/"+get_date()+".csv"
    opened_at = os.popen(" tail -n1 "+ filename).read().split("\t")[0]
    print(opened_at)
