# Watcher 

### Minimal Open source Screen-Time Tracker (CLI-app)

<img src="https://github.com/user-attachments/assets/e690e23c-3d09-4c23-b490-5cc8eff1a96b" width=90% height=auto>


## Table of Contents

- [About](#about)
- [Gallery](#gallery)
- [Installation](#installation)
- [Usage](#usage)
- [Want to Contribute](#want-to-contribute)
- [Todo](#to-do)


## About

Watcher is a CLI application written in Go, which helps you to get perspective about your Screen-time.

## Gallery


|                                          Day Summary                                          |                                         Week Summary                                          |
| :-------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------: |
| ![](https://github.com/user-attachments/assets/d7513cf1-f53e-429e-a1de-748bf4e0f094) | ![](https://github.com/user-attachments/assets/d44089ed-836b-4eee-b053-7f3adb850ec1) |

Funfact: The AFK (Away From Keyboard) feature helps make screen time tracking more accurate by pausing logging when you're idle (unless audio is playing)!

## Installation

**Dependencies:**

Before installing Watcher, please ensure you have the following dependencies installed:
*   **Go:** The Go programming language compiler and tools are required to build the application. You can find installation instructions at [golang.org/doc/install](https://golang.org/doc/install).
*   **xprintidle:** Used to get user idle time.
*   **xdotool:** Used to get active window information.

You can typically install `xprintidle` and `xdotool` on Debian/Ubuntu based systems using:
```bash
sudo apt-get update
sudo apt-get install xprintidle xdotool
```
For other distributions, please use your system's package manager.

**Installation Methods:**

1.  **Using the remote install script (recommended):**
    This command downloads and executes the main installation script directly.
    ```bash
    bash <(curl -s https://raw.githubusercontent.com/Waishnav/Watcher/main/install)
    ```

2.  **Manual installation from source:**
    First, clone the repository:
    ```bash
    git clone https://github.com/Waishnav/Watcher.git
    cd Watcher
    ```
    Then, make the install script executable and run it:
    ```bash
    chmod +x ./install
    ./install
    ```
    The `install` script will compile the Go application and place the binary in `/usr/local/bin`.

## Usage

Watcher provides the following commands:

*   **`watcher start`**:
    Starts the Watcher logging service. This command runs in the foreground and continuously monitors your application usage and AFK status, saving data to daily log files. You'll typically want to run this automatically on system startup (the `install` script attempts to set this up for X11 sessions via `.xprofile`).

*   **`watcher summary [argument]`**:
    Displays screen time summaries.
    *   If no argument is provided or if the argument is `today`, it shows the summary for the current day.
        ```bash
        watcher summary
        watcher summary today
        ```
    *   If the argument is `week`, it shows the summary for the current week.
        ```bash
        watcher summary week
        ```
    *   To show a summary for a specific date, use `YYYY-MM-DD` format:
        ```bash
        watcher summary 2023-10-27
        ```
    *   To show a summary for a specific week, use `Www-YYYY` format (e.g., W42-2023):
        ```bash
        watcher summary W43-2023
        ```

*   **`watcher --help` or `watcher [command] --help`**:
    Shows help information for the application or a specific command.

## Want to Contribute
If you are interested in contributing, please check out [CONTRIBUTING.md](https://github.com/Waishnav/Watcher/blob/main/CONTRIBUTING.md).

The project is now primarily developed in Go. Ensure you have a Go development environment set up.
The older Python versions/branches (v1.0, v2.0) are now considered legacy. Contributions should target the current Go codebase in the main branch.

You can also contribute to the [Watcher Website](https://github.com/Waishnav/Watcher-web) (made with React).

## To-do

- [x] AFK feature (Implemented in the Go version)
- [ ] GUI only if got 300 stars Probably [Tauri App](https://github.com/tauri-apps/tauri).

```
