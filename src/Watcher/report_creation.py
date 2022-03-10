import csv
import os
import time_operations as to
from log_files import get_date
from colored_text import Color

def imp(date):
    user = os.getlogin()
    path = "/home/" + user +"/.cache/Watcher/raw_data/"
    filename = path + date + ".csv"
    l = list() # l = list of  (app_name, time spent on app on that session)
    d = dict()

    with open(filename, 'r') as file:
        reader = csv.reader(file)
        for row in reader:
            for column in row:
                l.append([column[18::],column[9:18]])
                d.update({column[18::]: "O"})

    d = list(d) # list of app opened on that day
    return d, l

# creating dictionary to store time spend on each applicaitons on that particular day
def final_report(window_opened, time_spent):
    report = dict()

    for i in window_opened: # i is applications name
        time = '00:00:00'
        for j in time_spent: # j is list of applicaions_name and  time_spent in particular session
            if i == j[0]:
                time = to.time_addition(j[1], time)
                report.update({i:time})
    report.pop("User-loged-in")
    return report

# ░ ▒ █ ───
#print("▒▒▒\t▒▒▒\n███")

def prints_report(window_opened, time_spent):
    Total_screen_time = "00:00:00"
    for x,y in final_report(window_opened, time_spent).items():
        Total_screen_time = to.time_addition(y, Total_screen_time)
    print(Color.YELLOW("\n  Today's Screen-Time \t\t  ") + Color.BLUE(to.convert_time(Total_screen_time)))
    print("─────────────────────────────────────────────────")
    print(Color.RED(f'{"App Usages":>28}'))
    print("─────────────────────────────────────────────────")

    for x,y in final_report(window_opened, time_spent).items():
        if x == "":
            x = "Home-Screen"
        print("  " + Color.GREEN(f'{x:<21}') + '\t\t ',to.convert_time(y))

if __name__ == "__main__":
    date = get_date()
    window_opened, time_spent = imp(date)
    prints_report(window_opened, time_spent)

