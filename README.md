# Watcher

### Minimal Open source Screen-Time Tracker (CLI-app)

<img src="https://cdn.discordapp.com/attachments/846673042893832195/952274599276580874/watcher.png" width=90% height=auto>

## Table of Contents

- [About](#about)
- [Gallery](#gallery)
- [Installation](#installation)
- [Todo](#to-do)

## About

Watcher is CLI-app (at this moment) which helps you to get perspective about your Screen-time

## Gallery

|                                          Day Summary                                          |                                         Week Summary                                          |
| :-------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------: |
| ![](https://cdn.discordapp.com/attachments/846673042893832195/952283314746691624/unknown.png) | ![](https://cdn.discordapp.com/attachments/846673042893832195/952283190716948521/unknown.png) |

Funfact: You might be thinking how can someone has 14 hrs of screen time in a single day, Well ! short ans is AFK-feature is not implemented yet... Most of the time I left my laptop as it is so it also counts that AFK time as Screen-time

## Installation

- Note: Install [`xprintidle`](https://github.com/g0hl1n/xprintidle) and [`xdotool`](https://github.com/jordansissel/xdotool) on your system ( the only dependancies other than python3 ). Install [`python3`](https://www.python.org/downloads/) if not installed in your machine.
- First, Install the following dependancy `xprintidle` and `xdotool`

```bash
$ sudo [package-manager] install xprintidle xdotool
```

- Second, Clone this repository and cd into it-

```bash
$ git clone https://github.com/Waishnav/Watcher
$ cd ./Watcher/
```

- Then run install script

```bash
$ chmod +x ./install && ./install
```

## To-do

- [x] AFK feature
- [ ] GUI only if got 300 stars Probably [Tauri App](https://github.com/tauri-apps/tauri).
