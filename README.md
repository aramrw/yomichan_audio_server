_(iOS compatible audio files)_
### Add Audio Server to Yomichan/Yomitan
```
http://localhost:8080/?term={term}&reading={reading}
``` 
- Copy the above link & paste into the url box just like in the gif below 👇
<img  src="https://github.com/aramrw/yomichan_audio_server/assets/106574385/0f399e59-f3d4-4b6b-a54e-6daceb6bc582" width="400" />

### Installation (Linux + MacOS + Windows)
- Download **[the latest yas exe](https://github.com/aramrw/yomichan_audio_server/releases/latest)** & put the exe inside any folder
- Also download at least one audio folder from the **[releases page](https://github.com/aramrw/yomichan_audio_server/releases/latest)**.
- **Pro Tip:** For a massive collection of 1.2M+ files, check out the **[Ultimate Audio Source](https://github.com/aramrw/yomichan_audio_server/issues/13)** (requires [7-Zip](https://www.7-zip.org/) to extract).
- Create an `audio/` folder and put the audio files inside that folder.
Make sure it looks like this 👇
yomichan_audio_server_v0.1.2/ <- this can be any folder
├── audio/
│   ├── daijisen/media
│   ├── jpod/media
│   ├── nhk16/media
│   ├── shinmeikai8/media
│   ├── forvo_jp/
│   ├── forvo_zh/
│   ├── ozk5_files/       <-- New sources
│   ├── taas_files/
├── yomichan_audio_server.exe
├── entries.db            <-- Database file
```

### Ultimate Audio Source Setup (Important!)
If you are using the massive "Ultimate Audio Source" pack:
1. Extract the zip contents into your `audio/` folder.
2. The zip includes an `entry_and_pitch_db.sql` file. You **MUST** import this into your `entries.db` for the server to recognize the new files.
   - Install **[SQLite](https://sqlite.org/download.html)**.
   - Open a terminal in the folder and run:
     ```bash
     sqlite3 entries.db ".read entry_and_pitch_db.sql"
     ```
   - This may take 10-20 minutes depending on your disk speed.
3. Once finished, ensure `entries.db` is present next to the executable.
### Sorting
- create a `sort.txt` file where the exe is
- run program with `--sources` to see sources list
- add at least 1 source on each line

**Example `sort.txt`:**
```
nhk16
daijisen
shinmeikai8
forvo_jp
jpod
```
This will prioritize NHK16 audio first, then Daijisen, etc.

### Performance
- On first startup, the server builds a file cache of all audio files, which is saved to `audio_cache.json`
- Subsequent startups load from the cache for faster initialization
- **The cache automatically detects when audio files are added/removed and rebuilds itself** - no manual intervention needed!

### Running on Startup
**Windows (Task Scheduler - runs in background):**
```powershell
schtasks /create /tn "Yomichan Audio Server" /tr "C:\path\to\yomichan_audio_server.exe --log headless" /sc onlogon /rl highest
```
Replace `C:\path\to\` with the actual path to your exe.

**Linux (systemd):**
Create `~/.config/systemd/user/yomichan-audio.service`:
```ini
[Unit]
Description=Yomichan Audio Server

[Service]
ExecStart=/path/to/yomichan_audio_server --log headless
Restart=on-failure

[Install]
WantedBy=default.target
```
Then run:
```bash
systemctl --user enable yomichan-audio.service
systemctl --user start yomichan-audio.service
```

**macOS (launchd):**
Create `~/Library/LaunchAgents/com.yomichan.audioserver.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.yomichan.audioserver</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/yomichan_audio_server</string>
        <string>--log</string>
        <string>headless</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```
Then run:
```bash
launchctl load ~/Library/LaunchAgents/com.yomichan.audioserver.plist
```
### Issues: 
- If you are having problems, run the program with `--log full`
- Make sure to include the operating system and send bug reports in **[Issues](https://github.com/aramrw/yomichan_audio_server/issues)**.
