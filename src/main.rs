extern crate hyper_tls;
extern crate futures;
extern crate tokio_core;
extern crate base64;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate hyper;
extern crate url;

extern crate clap;

use url::Url;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

use std::io::{Write, BufWriter};
use serde_json::Error;
use base64::decode;

use clap::{Arg, App};

#[derive(Deserialize,Debug)]
struct Clip {
    clip_id: String,
    base_url: String,
    video: Vec<Video>,
    audio: Vec<Audio>,
}

impl Clip {
    fn new(data: &str) -> Result<Self, Error> {
        serde_json::from_str::<Self>(data)
    }

    fn video_list(&self, url: &str) -> (Vec<u8>, Vec<Url>){
        if let Some(v) = self.video.iter().max_by_key(|x| x.bitrate) {
            println!("Select maximum bitrate stream: {}", v.bitrate);
            let p = Url::parse(url).unwrap()
                .join(&self.base_url).unwrap()
                .join(&v.base_url).unwrap();

            ( match decode(&v.init_segment){
                Ok(v) => v,
                Err(_) => Vec::new(),
            },
              v.segments.iter().map(|x| { p.join(&x.url).unwrap() }).collect::<Vec<Url>>()
            ) 
        } else {
            (Vec::new(), Vec::new())
        }
    }
    
    fn audio_list(&self, url: &str) -> (Vec<u8>, Vec<Url>){
        if let Some(v) = self.audio.iter().max_by_key(|x| x.bitrate) {
            println!("Select maximum bitrate stream: {}", v.bitrate);
            let p = Url::parse(url).unwrap()
                .join(&self.base_url).unwrap()
                .join(&v.base_url).unwrap();

            ( match decode(&v.init_segment){
                Ok(v) => v,
                Err(_) => Vec::new(),
            },
              v.segments.iter().map(|x| { p.join(&x.url).unwrap()}).collect::<Vec<Url>>()
            ) 
        } else {
            (Vec::new(), Vec::new())
        }

    }
}

#[derive(Deserialize,Debug)]
struct Video {
    id: String,
    base_url: String,
    format: String,
    mime_type: String,
    codecs: String,
    bitrate: u64,
    avg_bitrate: u64,
    duration: f64,
    framerate: f64,
    width: u32, height: u32,
    max_segment_duration: u64,
    init_segment: String,
    segments: Vec<Segment>,
}

#[derive(Deserialize,Debug)]
struct Segment {
    start: f64,
    end: f64,
    url: String,
}

#[derive(Deserialize,Debug)]
struct Audio {
    id: String,
    base_url: String,
    format: String,
    mime_type: String,
    codecs: String,
    bitrate: u64,
    avg_bitrate: u64,
    duration: f64,
    channels: u32,
    sample_rate: u64,
    max_segment_duration: u64,
    init_segment: String, 
    segments: Vec<Segment>,
}


fn main() {
    let args = App::new("Vimeo Downloader")
        .version("0.1")
        .author("Hongjie Zhai <zhaihj@live.jp")
        .about("download vimeo segmented video (master.json)")
        .arg(Arg::with_name("output")
             .short("o")
             .long("output")
             .value_name("FILE")
             .required(true)
             .use_delimiter(false)
             .help(r#"Set output file path.
you will get two file: FILE_v.mp4 and FILE_a.mp3
use 
ffmpeg -i FILE_v.mp4 -i FILE_a.mp3 -acodec copy -vcodec copy FILE.mp4
to merge them"#)
             .takes_value(true))
        .arg(Arg::with_name("url")
             .short("u")
             .long("url")
             .required(true)
             .use_delimiter(false)
             .value_name("URL")
             .help("Video url that usually ends with `master.json?base64_init=1`")
             .takes_value(true))
        .get_matches();
    if let Some(url) = args.value_of("url") {
        let output_file = args.value_of("output").unwrap_or("vimeo_video").to_string();

        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let client = hyper::Client::configure()
            .connector(hyper_tls::HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);
        println!("url: {}", url.parse::<hyper::Uri>().unwrap());
        let work_master = client.get(url.parse::<hyper::Uri>().unwrap()).and_then(|res| {
            // get master.json
            res.body().concat2().and_then(|body| Ok(String::from_utf8(body.to_vec()).unwrap()))
        });
        let s = core.run(work_master).unwrap();
        let clip = Clip::new(&s).unwrap();
        // get segments
        let (vh, vl) = clip.video_list(&url);

        let output = std::fs::File::create(output_file.clone() + "_v.mp4").unwrap();
        let mut output = BufWriter::new(output);
        output.write_all(&vh).unwrap();

        println!("video: {}\n{}", vl.len(), "-".repeat(vl.len()));
        for i in vl {
            print!("-");
            std::io::stdout().flush().unwrap();
            let w = client.get(i.as_str().parse::<hyper::Uri>().unwrap())
                .and_then(|res| {
                    res.body().concat2()
                    .and_then(|body| 
                    Ok(output.write_all(&body).unwrap())
                    )
                });
            core.run(w).unwrap();
        }
        println!("");

        let (ah, al) = clip.audio_list(&url);
        let output = std::fs::File::create(output_file.clone() + "_a.mp3").unwrap();
        let mut output = BufWriter::new(output);
        output.write_all(&ah).unwrap();

        println!("audio: {}\n{}", al.len(), "-".repeat(al.len()));
        for i in al {
            print!("-");
            std::io::stdout().flush().unwrap();
            let w = client.get(i.as_str().parse::<hyper::Uri>().unwrap())
                .and_then(|res| {
                    res.body().concat2()
                    .and_then(|body| 
                    Ok(output.write_all(&body).unwrap())
                    )
                });
            core.run(w).unwrap();
        };
        println!("");
        println!("All finished!\nPlease use ffmpeg -acodec copy -vcodec copy to merge the a/v stream.");
    }
}
