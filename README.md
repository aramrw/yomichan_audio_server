### Sunset (November 4, 2026)
> Because of the lack of consistent ways maintain and index large amounts of audio files for multiple languages, I'm gonna sunset this project to focus on my other projects that are more multi-language friendly instead of just Japanese.
>
> Even though this project works with any language, online scrapes of places like forvo are not consistently maintained with proper index files, or completely ambigous, if their metadata is lost the files are useless if not properly descripted. I recommend either forking it and maintaing your own fork, or using other projects such as the [Yomitan Ultimate Audio Source](https://animecards.site/yomitan_audio/), or rewriting it from scratch (no mention necessary). Sorry.
>
> I may un-sunset it in the future if it becomes easier to download consistent high quality audio across multiple languages, or if you know a source that does so, please let me know.

---

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
### Sorting
- create a `sort.txt` file where the exe is
- run program with `--sources` to see sources list
- add at least 1 source on each line
### Issues: 
- If you are having problems, run the program with `--log full`
- Make sure to include the operating system and send bug reports in **[Issues](https://github.com/aramrw/yomichan_audio_server/issues)**.
