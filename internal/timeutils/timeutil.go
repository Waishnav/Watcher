// format the seconds to HH:MM:SS format
package timeutils

import "time"

func FormatSeconds(seconds int64) string {
	return time.Unix(seconds, 0).Format("15:04:05")
}

func GetDate() string {
	return time.Now().Format("2006-01-02")
}

func Sleep(seconds int64) {
	time.Sleep(time.Duration(seconds) * time.Second)
}
