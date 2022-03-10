
def time_difference(a,b): # b - a
    hr = int(b[0:2])-int(a[0:2])
    mn = int(b[3:5])-int(a[3:5])
    sec = int(b[6:8])-int(a[6:8])
    if int(mn) < 0 and int(sec) < 0:
        hr = hr - 1
        mn = 60 + mn - 1
        sec = 60 + sec
        if hr < 0:
            hr = hr + 24
    elif int(mn) < 0 and int(sec) > 0:
        hr = hr - 1
        mn = 60 + mn
    elif int(sec) < 0 and int(mn) > 0:
        sec = 60 + sec
        mn = mn - 1

    hr = str(hr).zfill(2)
    mn = str(mn).zfill(2)
    sec = str(sec).zfill(2)
    result = hr + ":" + mn + ":" + sec

    return result

def time_addition(a,b):
    hr = int(b[0:2]) + int(a[0:2])
    mn = int(b[3:5]) + int(a[3:5])
    sec = int(b[6:8]) + int(a[6:8])
    if mn >= 60 and sec >= 60:
        hr = hr + 1
        mn = mn - 60 + 1
        sec = sec - 60
    elif mn >= 60:
        hr = hr + 1
        mn = mn - 60
    elif sec >= 60:
        mn = mn + 1
        sec = sec - 60

    hr = str(hr).zfill(2)
    mn = str(mn).zfill(2)
    sec = str(sec).zfill(2)
    result = hr + ":" + mn + ":" + sec

    return result

def convert_time(t):
    if int(t[0:2]) == 0:
        result = t[3:5] + 'm ' + t[6::] + 's'
        if int(t[3:5]) == 0:
            result = t[6::] + 's'
    else:
        result =  t[0:2] + 'h ' + t[3:5] + 'm ' + t[6::] + 's'
    return result
