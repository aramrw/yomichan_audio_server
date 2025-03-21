### Features
- Instant audio lookup speeds.
- Tiny memory usage
- iOS compatible audio files.
### Included Audio Files 🔉
- **`NHK16 日本語発音辞典`** (~**1.29** GB) - _**102,823** files_
- **`Jpod`** (~**1.63** GB) - _**134,103** files_
- **`大辞泉`** (~**908** MB) - _**61,521** files_
- **`Forvo`** (~**717** MB) - _**78,835** files_
- **`新明解８版`** (~**588** MB) - _**66,726** files_
### Add Audio Server to Yomichan/Yomitan
```
http://localhost:8080/?term={term}&reading={reading}
``` 
- Copy the above link & paste into the url box just like in the gif below 👇
<img  src="https://github.com/aramrw/yomichan_audio_server/assets/106574385/0f399e59-f3d4-4b6b-a54e-6daceb6bc582" width="600" />

### Installation (Linux + MacOS + Windows)
- Download **[the latest yas exe for your system.](https://github.com/aramrw/yomichan_audio_server/releases/latest)**.
- Put the exe inside any folder
- Also download the audio files from the **[releases page](https://github.com/aramrw/yomichan_audio_server/releases/latest)**.
- Create an `audio/` folder and put the audio files inside that folder.
Make sure it looks like this 👇
```
yomichan_audio_server_v0.1.2/ <- this can be any folder
├── audio/
│   ├── daijisen/media
│   ├── jpod/media
│   ├── nhk16/media
│   ├── shinmeikai8/media
│   ├── forvo_jp/
│   ├── forvo_zh/
├── yomichan_audio_server.exe
```
#### Debug: 
- If you are having problems, click on the `Debug` menu item in the system tray, and check error messages.
- Send bug reports in **[Issues](https://github.com/aramrw/yomichan_audio_server/issues)**.

