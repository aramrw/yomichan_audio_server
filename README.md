### Features
- Instant Lookup
- Tiny Memory Usage
- iOS Compatible
### Included Audio Files 🔉
- **`NHK16 日本語発音辞典`** (~**1.29** GB) - _**102,823** files_
- **`Jpod`** (~**1.63** GB) - _**134,103** files_
- **`大辞泉`** (~**908** MB) - _**61,521** files_
- **`Forvo`** (~**717** MB) - _**78,835** files_
- **`新明解８版`** (~**588** MB) - _**66,726** files_
### Installation & Use
- Download **`yomichan_audio_server_v0.3.0.rar`** from the **[Releases Page](https://github.com/aramrw/yomichan_audio_server/releases/tag/v0.3.0)**.
- Download the audio files you want _(all recommended)_ from the **[Releases Page](https://github.com/aramrw/yomichan_audio_server/releases/tag/v0.3.0)**.
- Extract and place the audio files folders inside **`yomichan_audio_server_v0.3.0/audio`**. It should look like this 👇
```
yomichan_audio_server_v0.3.0\
├── audio/
│   ├── daijisen_files\
│   ├── forvo_files\
│   ├── jpod_files\
│   ├── nhk16_files\
│   ├── shinmeikai8＿files\
│   ├── entries.db
│   ├── entries.v
├── config.json
├── yomichan_audio_server.exe
```
### Config Settings
#### exit_minutes:
- Default is 60 minutes _(adjusting it higher is recommended)_.
#### debug: 
- If you are having problems, set to true to unhide terminal and check error messages. Send bug reports in **[Issues](https://github.com/aramrw/yomichan_audio_server/issues)**.
```
{
  "exit_minutes": 60,
  "debug": false
}
```
