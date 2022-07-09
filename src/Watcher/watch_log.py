import os
import csv
import time
import get_windows as x
import afk
from time_operations import time_difference, time_addition

# get current time whenever the function is called
def get_time():
    t = os.popen('''date +"%T"''').read()
    return t[0:-1]

# get date of today
def get_date():
    d = os.popen('''date +"%Y-%m-%d"''').read()
    return d[0:-1]

def append_line_in_csv(date, opened_time, time_spent, window_name):
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/raw_data/"+date+".csv"
    Data = [opened_time, time_spent,  window_name]
    with open(filename, 'a') as csvfile:
        csvwriter = csv.writer(csvfile, delimiter='\t')
        csvwriter.writerow(Data)

# Expected Behaviour == if date got changed then append line in new csv file after initializing the csv file

def log_creation():
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/raw_data/"+get_date()+".csv"
    if not(os.path.isfile(filename)):
        with open(filename, 'a') as csvfile:
            csvwriter = csv.writer(csvfile, delimiter='\t')
            csvwriter.writerow([get_time(), "00:00:00", ""])

    # appending line at login
    logged_out_time = os.popen("tail -n1 " + filename).read().split("\t")[0]
    time_away_from_laptop = time_difference(logged_out_time, get_time())
    append_line_in_csv(get_date(), get_time(), time_away_from_laptop, "User-logged-in")

    while True:
        actv_window = x.active_window()
        if x.is_window_changed(actv_window):
            next_actv_window = x.active_window()
            date = get_date()
            opened_at = get_time()
            time.sleep(0.5)
            time_spent = "00:00:01"
            append_line_in_csv(date, opened_at, time_spent, next_actv_window)

        else:
            actv_window = x.active_window()
            date = get_date()
            now = get_time() # for next_window its the opening time
            last_line = os.popen("tail -n1 " + filename).read().split("\t")
            opened_at = last_line[0]
            os.popen("""sed -i -e '$ d' """ + filename)
            time.sleep(1)
            time_spnt = time_difference(opened_at, get_time())

            if afk.IsAFK():
                append_line_in_csv(date, opened_time, time_spent, "AFK")
            else:
                append_line_in_csv(date, opened_at, time_spnt, actv_window)

if __name__ == "__main__":
    opened_at = os.popen("""tail -n1 ~/.cache/Watcher/raw_data/2022-07-07.csv""").read().split("\t")[1]
    print(opened_at)
