# Watcher

Watcher measures time spent interacting with foreground applications on a personal computer.

## Language

**Observation**:
A time-stamped signal from a collector about focus, input-idle state, session lifecycle, or another observable change. Observations may arrive late or be duplicated.
_Avoid_: Screen-time entry

**Collector**:
An operating-system or compositor integration that reports observations and its supported capabilities. Its availability is independent of the analytics engine.
_Avoid_: Tracker

**Foreground application**:
The application the desktop considers focused at a point in time. It can be unknown even while the desktop session is running.
_Avoid_: Productive app

**Activity interval**:
A half-open period of time with a known foreground application, activity state, and source attribution. Intervals are the canonical units used to calculate usage totals.
_Avoid_: Log line, Daily total

**Active interaction time**:
The total non-idle time attributed to foreground applications within a requested range, limited by the confidence of observed input-idle state. It is not a measure of attention or productivity.
_Avoid_: Focus time, Attention time

**Idle time**:
A period in which the OS/compositor reports that there has been no qualifying user input for the configured threshold.
_Avoid_: Break time

**Passive media time**:
Time associated with observed media playback while the user is idle, when a suitable signal is available. It is reported separately and is not assumed to imply attention.
_Avoid_: Active usage

**Capability**:
A specific type of observation a collector can provide in the current environment, including access-denied and unsupported states.
_Avoid_: Platform support

**Coverage gap**:
A period where activity cannot be observed reliably, such as collector disconnection or unsupported idle detection. A gap must not be counted as active time.
_Avoid_: Zero usage
