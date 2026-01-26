use crate::settings;
use random::Source;
use rodio::OutputStreamBuilder;
use std::fs;
use std::io::BufReader;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const SUPPORTED_AUDIO_EXTENSIONS: [&str; 4] = ["mp3", "wav", "flac", "ogg"];

pub fn meow() {
    // need a thread to not block the UI
    thread::spawn(|| {
        let stream_handle = OutputStreamBuilder::open_default_stream()
            .expect("Failed to open default audio stream");

        // read audio directory
        let settings = settings::GlobalSettings::load().expect("Failed to load settings");
        let audio_dir = settings.audio_directory.join("audio");
        let audio_files = fs::read_dir(audio_dir).expect("Failed to read audio directory");

        // filter by supported audio extensions
        let audio_files = audio_files
            .filter(|entry| {
                entry.as_ref().is_ok_and(|e| {
                    SUPPORTED_AUDIO_EXTENSIONS
                        .iter()
                        .any(|ext| e.file_name().to_string_lossy().ends_with(ext))
                })
            })
            .collect::<Vec<_>>();

        // calculate random index
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get unix time");
        let mut source = random::default(unix_time.as_micros() as u64);
        let index = source.read_u64() % audio_files.len() as u64;

        // select audio file
        let audio_file = audio_files[index as usize]
            .as_ref()
            .expect("Failed to get audio file");

        // play audio file
        let handle = fs::File::open(audio_file.path()).expect("Failed to open audio file");
        let file = BufReader::new(handle);
        let sink = rodio::play(&stream_handle.mixer(), file).unwrap();
        sink.sleep_until_end();
    });
}
