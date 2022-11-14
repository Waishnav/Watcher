import os
import csv
from watch_log import get_date
import datetime
import time_operations as to

# creating dictionary to store time spend on each applicaitons on that particular day
def final_report(date):
    path = "/home/" + os.getlogin() +"/.cache/Watcher/daily_data/"
    filename = path + date + ".csv"

    report = dict()
    if os.path.isfile(filename):
        with open(filename, 'r') as f:
            raw_data = f.readlines()
            for x in raw_data:
                x = x.split('\t')
                a = {x[1][:-1]:x[0]}
                report.update(a)

    #print(report)
    if "Unknown" in report.keys():
        report.pop("Unknown")
    return report

def sort_data(report):
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
    del report
    del sorted_values
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

def weekly_logs(week = str(os.popen('''date +"W%V-%Y"''').read()[0:-1])):
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/Analysis/"+week+".csv"
    with open(filename, "w") as csvfile:
        csvwriter = csv.writer(csvfile, delimiter='\t')

        if os.popen('''date +"W%V-%Y"''').read()[0:-1] == week:
            dates = week_dates()
        else:
            dates = week_dates(get_sunday_of_week(week))

        for i in dates:
            Total_screen_time = "00:00:00"
            for x, y in final_report(i).items():
                Total_screen_time = to.time_addition(y, Total_screen_time)
            csvwriter.writerow([weekday_from_date(i), Total_screen_time])

        all_data = dict()
        for i in dates:
            for x, y in final_report(i).items():
                usage = all_data.get(x)
                if usage == None:
                    usage = "00:00:00"
                y = to.time_addition(usage, y)
                all_data.update({x:y})

        all_data = sort_data(all_data)
        for x, y in all_data.items():
            csvwriter.writerow([y, x])

#testing
if __name__ == "__main__":
    weekly_logs("W29-2022")

