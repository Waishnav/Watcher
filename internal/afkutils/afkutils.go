package afkutils

import (
	"fmt"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

func IsAFK(timeout int) bool {
	timeoutMilliseconds := time.Duration(timeout) * time.Minute

	timeoutLastInput := GetTimeSinceLastInput()

	return timeoutLastInput > timeoutMilliseconds
}

func GetTimeSinceLastInput() time.Duration {
	output, err := exec.Command("xprintidle").Output()
	if err != nil {
		fmt.Println("Error getting idle time")
	}

	inputMillis := strings.TrimSpace(string(output))
	inputMillisInt, err := strconv.Atoi(inputMillis)

	return time.Duration(inputMillisInt) * time.Millisecond
}

func ReturnedFromAFK(afkActive bool, timeout int) bool {
	return afkActive && !IsAFK(timeout)
}
