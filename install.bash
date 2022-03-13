sudo cp -r ./src/Watcher /usr/share/
cp -r ./src/bin/watcher ~/.local/bin/
sudo chmod +x ~/.local/bin/watcher
cp -r ./src/service/watcher.service ~/.config/systemd/user/
systemctl --user enable watcher.service
systemctl --user start watcher.service

mkdir ~/.cache/Watcher/
mkdir ~/.cache/Watcher/raw_data/
mkdir ~/.cache/Watcher/Analysis/

