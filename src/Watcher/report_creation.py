import csv
import os
import time_operations as to
from watch_log import get_date
from colored_text import Color
import datetime

def extract_data(date):
    user = os.getlogin()
    path = "/home/" + user +"/.cache/Watcher/raw_data/"
    filename = path + date + ".csv"

    l = list() # l = list of  (app_name, time spent on app on that session)
    d = dict()

    if os.path.isfile(filename):
        with open(filename, 'r') as file:
            reader = csv.reader(file)
            for row in reader:
                for column in row:
                    l.append([column[18::],column[9:17]])
                    d.update({column[18::]: "O"})
    else:
        None

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

    #print(report)
    if "User-logged-in" in report.keys():
        report.pop("User-logged-in")
    # sort report dictonary in decreasing order of Usages
    sorted_values = []
    for x,y in report.items():
        sorted_values.append(to.convert_into_sec(y))

    sorted_values.sort(reverse=True)
    sorted_report = dict()

    for i in sorted_values:
        for x, y in report.items():
            if to.convert_into_sec(y) == i:
                sorted_report.update({x:y})

    return sorted_report

# ░ ▒ █ ───
#print("▒▒▒\t▒▒▒\n███")

def prints_report(window_opened, time_spent):
    Total_screen_time = "00:00:00"
    for x,y in final_report(window_opened, time_spent).items():
        Total_screen_time = to.time_addition(y, Total_screen_time)

    if len(to.format_time(Total_screen_time)) == 3:
        print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>16}'))
    elif len(to.format_time(Total_screen_time)) == 7:
        print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>11}'))
    elif len(to.format_time(Total_screen_time)) == 11:
            print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(to.format_time(Total_screen_time)))

    print(" ────────────────────────────────────────────────")
    print(Color.RED(f'{" App Usages":>29}'))
    print(" ────────────────────────────────────────────────")

    for x,y in final_report(window_opened, time_spent).items():
        if x == "":
            x = "Home-Screen"
        print("   " + Color.GREEN(f'{x:<22}') + '\t ',f'{to.format_time(y):>12}')


