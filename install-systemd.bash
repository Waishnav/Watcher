#!/bin/bash

sudo cp -r ./src/Watcher /usr/share/
mkdir -p ~/.local/bin/
cp -r ./src/bin/watcher ~/.local/bin/
sudo chmod +x ~/.local/bin/watcher

# enabling systemd service
mkdir -p ~/.config/systemd/
mkdir -p ~/.config/systemd/user/
cp -r ./src/service/watcher.service ~/.config/systemd/user/
systemctl --user enable --now watcher.service

# making directory for log-files (where all you daily logs are stored)
mkdir -p ~/.cache/Watcher/
mkdir -p ~/.cache/Watcher/raw_data/
mkdir -p ~/.cache/Watcher/Analysis/

