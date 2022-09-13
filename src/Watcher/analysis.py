import os
import csv
from watch_log import get_date
import datetime
import time_operations as to

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
    if "AFK" in report.keys():
        report.pop("AFK")
    if "Unknown" in report.keys():
        report.pop("Unknown")
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

def get_sunday_of_week(week):
    year = int(week[4:])
    week = int(week[1:3])
    first = datetime.date(year, 1, 1)
    base = 1 if first.isocalendar()[1] == 1 else 8
    return first + datetime.timedelta(days=base - first.isocalendar()[2] + 7 * (week - 1)) + datetime.timedelta(days=6.9)

# getting dates of particular week for week summary
def week_dates(theday=datetime.date.today()):
    weekday = theday.isoweekday() - 1
    # The start of the week (Monday)
    start = theday - datetime.timedelta(days=weekday)
    # build a simple range
    dates = [start + datetime.timedelta(days=d) for d in range(weekday + 1)]
    dates = [str(d) for d in dates]
    return dates

def weekday_from_date(date):
    day = os.popen('''date -d "'''+ date + '''" +%a''').read()
    return day[0:-1]

def get_total_screen_time_for_day(date):
        window_opened, time_spent = extract_data(date)
        Total_screen_time = "00:00:00"
        for x, y in final_report(window_opened, time_spent).items():
            Total_screen_time = to.time_addition(y, Total_screen_time)
        return Total_screen_time


def weekly_logs(week = str(os.popen('''date +"W%V-%Y"''').read()[0:-1])):
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/Analysis/"+week+".csv"
    with open(filename, "w") as csvfile:
        csvwriter = csv.writer(csvfile, delimiter='\t')
        #csvwriter.writerow(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])

        if os.popen('''date +"W%V-%Y"''').read()[0:-1] == week:
            dates = week_dates()
        else:
            dates = week_dates(get_sunday_of_week(week))

        window_opened = list()
        time_spent = list()
        for i in dates:

            Total_screen_time = get_total_screen_time_for_day(i)

            csvwriter.writerow([weekday_from_date(i), Total_screen_time])

        for i in dates:
            x, y = extract_data(str(i))
            window_opened += x
            time_spent += y
        for x, y in final_report(window_opened, time_spent).items():
            csvwriter.writerow([y, x])

#testing
if __name__ == "__main__":
    weekly_logs("W29-2022")

