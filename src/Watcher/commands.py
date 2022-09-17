import os
import csv
import datetime
from watch_log import get_date
import analysis as anls
import time_operations as to

class Color:

    def GREY(text):
        return '\033[90m' + text + '\033[0m'

    def BLUE(text):
        return '\033[34m' + text + '\033[0m'

    def GREEN(text):
        return '\033[32m' + text + '\033[0m'

    def YELLOW(text):
        return '\033[33m' + text + '\033[0m'

    def RED(text):
        return '\033[31m' + text + '\033[0m'

    def PURPLE(text):
        return '\033[35m' + text + '\033[0m'

    def DARKCYAN(text):
        return '\033[36m' + text + '\033[0m'

    def BOLD(text):
        return '\033[1m' + text + '\033[0m'

    def UNDERLINE(text):
        return '\033[4m' + text + '\033[0m'

def daily_summary(date = get_date()):
    Total_screen_time = "00:00:00"
    for x,y in anls.final_report(date).items():
        Total_screen_time = to.time_addition(y, Total_screen_time)

    if date == get_date():
        if len(to.format_time(Total_screen_time)) == 3:
            print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>16}'))
        elif len(to.format_time(Total_screen_time)) == 7:
            print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>11}'))
        elif len(to.format_time(Total_screen_time)) == 11:
            print(Color.YELLOW("\n   Today's Screen-Time\t\t   ") + Color.BLUE(to.format_time(Total_screen_time)))
    elif date == os.popen("""date -d "1 day ago" '+%Y-%m-%d'""").read()[:-1]:
        if len(to.format_time(Total_screen_time)) == 3:
            print(Color.YELLOW("\n   Yestarday's Screen-Time\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>16}'))
        elif len(to.format_time(Total_screen_time)) == 7:
            print(Color.YELLOW("\n   Yestarday's Screen-Time\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>11}'))
        elif len(to.format_time(Total_screen_time)) == 11:
            print(Color.YELLOW("\n   Yestarday's Screen-Time\t   ") + Color.BLUE(to.format_time(Total_screen_time)))
    else:
        if len(to.format_time(Total_screen_time)) == 3:
            print(Color.YELLOW("\n   "+date+"'s Screen-Time\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>6}'))
        elif len(to.format_time(Total_screen_time)) == 7:
            print(Color.YELLOW("\n  "+ date+ "'s Screen-Time\t   ") + Color.BLUE(f'{to.format_time(Total_screen_time):>1}'))
        elif len(to.format_time(Total_screen_time)) == 11:
            print(Color.YELLOW("\n   "+date+"'s Screen-Time\t   ") + Color.BLUE(to.format_time(Total_screen_time)))

    print(" ────────────────────────────────────────────────")
    print(Color.RED(f'{" App Usages":>29}'))
    print(" ────────────────────────────────────────────────")

    for x,y in anls.sort_data(anls.final_report(date)).items():
        if x == "":
            x = "Home-Screen"
        print("   " + Color.GREEN(f'{x:<22}') + '\t ',f'{to.format_time(y):>12}')

def week_summary(week = os.popen('''date +"W%V-%Y"''').read()[:-1]):
    user = os.getlogin()
    filename = "/home/"+user+"/.cache/Watcher/Analysis/"+week+".csv"
    with open(filename, 'r') as file:
        csvreader = csv.reader(file, delimiter='\t')
        week_overview = dict()
        app_usages = dict()
        for row in csvreader:
            if len(row[0]) == 3:
                week_overview.update({row[0]:row[1]}) # Weekday -- screen-time
            else:
                app_usages.update({row[1]:row[0]}) # app-name -- usage

    week_screen_time = "00:00:00"
    for x, y in week_overview.items():
        week_screen_time = to.time_addition(y, week_screen_time)

    if week == os.popen('''date +"W%V-%Y"''').read()[:-1]:
        print(Color.PURPLE("\n   Week's screen-time\t\t   ") + Color.BLUE(to.format_time(week_screen_time)))
    elif week == os.popen("""date -d 'last week' '+W%W-%Y'""").read()[:-1]:
        print(Color.PURPLE("\n   Previous Week's \t\t   ") + Color.BLUE(to.format_time(week_screen_time)))
        print(Color.PURPLE("     Screen-Time"))
    else:
        print(Color.PURPLE("\n     "+week[1:3]+ "th week of\t   ") + Color.BLUE(to.format_time(week_screen_time)))
        print(Color.PURPLE("   "+week[4:] +" screen-time\t    "))

    print(" ────────────────────────────────────────────────")

    for x, y in week_overview.items():
        print("  " + f'{Color.YELLOW(x):>21}' + "\t\t   " + Color.BLUE(to.format_time(y)))

    #anls.prints_report(window_opened, time_spent, is_week)
    print(" ────────────────────────────────────────────────")
    print(Color.RED(f'{" App Usages":>29}'))
    print(" ────────────────────────────────────────────────")
    for x,y in app_usages.items():
        if x == "":
            x = "Home-Screen"
        print("   " + Color.GREEN(f'{x:<22}') + '\t ',f'{to.format_time(y):>12}')

#testing
if __name__ == "__main__":
    week_summary("W27-2022")
    #daily_summary("2022-07-18")
