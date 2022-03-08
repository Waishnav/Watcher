import csv
import os
import time_operations as to
from log_files import get_date

class color:
   GREY = '\033[30m'
   PURPLE = '\033[95m'
   CYAN = '\033[96m'
   DARKCYAN = '\033[36m'
   BLUE = '\033[34m'
   GREEN = '\033[32m'
   YELLOW = '\033[33m'
   RED = '\033[31m'
   BOLD = '\033[1m'
   UNDERLINE = '\033[4m'
   END = '\033[0m'

date = get_date()
def load_raw_data(date):
    user = os.getlogin()
    path = "/home/" + user +"/.cache/Watcher/raw_data/"
    filename = path + date + ".csv"
    l = list()
    d = dict()

    with open(filename, 'r') as file:
        reader = csv.reader(file)
        for row in reader:
            for column in row:
                l.append([column[0:8],column[9::]])
                d.update({column[9::]: "opened"})
                #print(column[0:8])
                #print(column[9::])

    d = list(d)
    return l, d
#print(raw_data_list[1][1]
#print(window_opened)
def each_session(raw_data_list):
    time_spent = []
    for i in range(len(raw_data_list)-1):
        a = raw_data_list[i][0]
        b = raw_data_list[i+1][0]
        time_spent.append([raw_data_list[i][1], to.time_difference(a, b)])

    return time_spent

# creating dictionary to store time spend on each applicaitons
def final_report(window_opened, time_spent):
    report = dict()

    for i in window_opened:
        time = '00:00:00'
        for j in time_spent:
            if i == j[0]:
                #print(j[to.1],i)
                time = to.time_addition(j[1], time)
                report.update({i:time})
    return report

# ░ ▒ █ ───
#print("▒▒▒\t▒▒▒\n███")

raw_data_list, window_opened = load_raw_data(date)
time_spent = each_session(raw_data_list)


if __name__ == "__main__":

    Total_screen_time = "00:00:00"
    for x,y in final_report(window_opened, time_spent).items():
        Total_screen_time = to.time_addition(y, Total_screen_time)
    print(color.YELLOW + "\n  Today's Screen-Time \t\t " + color.END + color.BLUE + to.convert_time(Total_screen_time)+ color.END)
    print("─────────────────────────────────────────────────")
    print(color.RED + f'{"App Usages":>28}' + color.END)
    print("─────────────────────────────────────────────────")

    for x,y in final_report(window_opened, time_spent).items():
        if x == "":
            x = "HOME-SCREEN"
        print(color.GREEN + "  "+f'{x:<21}' + color.END ,'\t',to.convert_time(y))

