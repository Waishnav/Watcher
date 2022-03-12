import os
import csv
import datetime
from watch_log import get_date
import report_creation as rc
from colored_text import Color
import time_operations as to

def daily_summary():
    date = get_date()
    window_opened, time_spent = rc.extract_data(date)
    rc.prints_report(window_opened, time_spent)

def week_summary():
    W_Y = os.popen('''date +"W%U-%Y"''').read()[0:-1]
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/Analysis/"+W_Y+".csv"
    with open(filename, 'r') as file:
        csvreader = csv.reader(file, delimiter='\t')
        week_overview = dict()
        app_usages = dict()
        for row in csvreader:
            if len(row[0]) == 3:
                week_overview.update({row[0]:row[1]})
            else:
                app_usages.update({row[1]:row[0]})

    week_screen_time = "00:00:00"
    for x, y in week_overview.items():
        week_screen_time = to.time_addition(y, week_screen_time)

    print(Color.PURPLE("\n   Week's screen-time\t\t   ") + Color.BLUE(to.format_time(week_screen_time)))
    print(" ────────────────────────────────────────────────")

    for x, y in week_overview.items():
        print("  " + f'{Color.YELLOW(x):>21}' + "\t\t   " + Color.BLUE(to.format_time(y)))

    #rc.prints_report(window_opened, time_spent, is_week)
    print(" ────────────────────────────────────────────────")
    print(Color.RED(f'{" App Usages":>29}'))
    print(" ────────────────────────────────────────────────")
    for x,y in app_usages.items():
        if x == "":
            x = "Home-Screen"
        print("   " + Color.GREEN(f'{x:<22}') + '\t ',f'{to.format_time(y):>12}')
