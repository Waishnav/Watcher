import os
import csv
import time
import get_windows as x
import afk as y
from time_operations import time_difference, time_addition, convert

# get current time whenever the function is called
def get_time():
    t = os.popen('''date +"%T"''').read()
    return t[0:-1]

# get date of today
def get_date():
    d = os.popen('''date +"%Y-%m-%d"''').read()
    return d[0:-1]

def update_csv(date, Data):
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+date+".csv"
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
    filename = "/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+get_date()+".csv"
    if not(os.path.isfile(filename)):
        creat_file = "/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+get_date()+".csv"
        with open(creat_file, 'w') as fp:
            pass

    afk = False
    afkTimeout = 1 # timeout in minutes
    data = import_data(filename)
    while True:
        date = get_date()
        filename = "/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+date+".csv"
        afk = y.is_afk(afkTimeout)
        print(data)

        if not(afk):
            active_window = x.active_window()
            usage = data.get(active_window)
            if usage == None:
                usage = "00:00:00"

            time.sleep(1)
            if y.is_afk(afkTimeout):
                afk_time = int(round(int(os.popen("xprintidle").read()[:-1])/1000, 0))
                usage = time_difference(convert(afk_time), usage)

            usage = time_addition("00:00:01", usage)
            data.update({active_window : usage})
            if os.path.isfile("/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+get_date()+".csv"):
                update_csv(get_date(), data)
            elif not(os.path.isfile("/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+get_date()+".csv")):
                new_filename = "/home/"+os.getlogin()+"/.cache/Watcher/daily_data/"+get_date()+".csv"
                with open(new_filename, 'w') as fp:
                    pass

                data.clear()
        

if __name__ == "__main__":
    log_creation()
    #afk_time = int(round(int(os.popen("xprintidle").read()[:-1])/1000, 0))
    #print(afk_time)

