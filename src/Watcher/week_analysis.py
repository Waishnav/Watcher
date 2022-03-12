import os
import csv
import datetime
import report_creation as rc
import time_operations as to

def get_dates():
    theday = datetime.date.today()
    weekday = theday.isoweekday() - 1
    # The start of the week
    start = theday - datetime.timedelta(days=weekday)
    # build a simple range
    dates = [start + datetime.timedelta(days=d) for d in range(weekday+1)]
    dates = [str(d) for d in dates]

    return dates

def weekday_from_date(date):
    day = os.popen('''date -d "'''+ date + '''" +%a''').read()
    return day[0:-1]

W_Y = os.popen('''date +"W%U-%Y"''').read()[0:-1]
user = os.getlogin()
filename = "/home/"+user+"/.cache/Watcher/Analysis/"+W_Y+".csv"
with open(filename, "w") as csvfile:
    csvwriter = csv.writer(csvfile, delimiter='\t')
    #csvwriter.writerow(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
    dates = get_dates()

    window_opened = list()
    time_spent = list()
    for i in dates:
        window_opened, time_spent = rc.extract_data(i)
        Total_screen_time = "00:00:00"
        for x, y in rc.final_report(window_opened, time_spent).items():
            Total_screen_time = to.time_addition(y, Total_screen_time)

        csvwriter.writerow([weekday_from_date(i), Total_screen_time])

    for i in dates:
        x, y = rc.extract_data(str(i))
        window_opened += x # smth is wrong here
        time_spent += y
    for x, y in rc.final_report(window_opened, time_spent).items():
        csvwriter.writerow([y, x])

