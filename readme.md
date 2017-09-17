Download vimeo segemnted video (master.json)


### Compile

```
cargo build
```

### usage

Download a video:

```
./target/debug/vimeo-downloader -u "url-ends-with-master.json" -o "output-file-without-extension"
```

Merge audio/video streams:

```
ffmpeg -i output-file_a.mp3 -i output-file_v.mp4 -acodec copy -v codec copy output-file.mp4
```
