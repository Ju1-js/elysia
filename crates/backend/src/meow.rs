use crate::settings;
use random::Source;
use rodio::OutputStreamBuilder;
use std::fs;
use std::io::{BufReader, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const MEOW1: &[u8] = include_bytes!("../../../assets/audio/meow1.mp3");
const MEOW2: &[u8] = include_bytes!("../../../assets/audio/meow2.mp3");
const MEOW3: &[u8] = include_bytes!("../../../assets/audio/meow3.mp3");
const MEOW4: &[u8] = include_bytes!("../../../assets/audio/meow4.mp3");
const MEOW5: &[u8] = include_bytes!("../../../assets/audio/meow5.mp3");

const BUNDLED_AUDIO: &[(&str, &[u8])] = &[
    ("meow1.mp3", MEOW1),
    ("meow2.mp3", MEOW2),
    ("meow3.mp3", MEOW3),
    ("meow4.mp3", MEOW4),
    ("meow5.mp3", MEOW5),
];

const MAX_CONCURRENT_MEOWS: usize = 3;
static ACTIVE_MEOWS: AtomicUsize = AtomicUsize::new(0);

fn ensure_audio_files(audio_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(audio_dir)?;
    
    for (filename, bytes) in BUNDLED_AUDIO {
        let file_path = audio_dir.join(filename);
        if !file_path.exists() {
            let mut file = fs::File::create(&file_path)?;
            file.write_all(bytes)?;
            eprintln!("[Audio] Extracted bundled audio: {filename}");
        }
    }
    
    Ok(())
}

pub fn meow() {
    // Check if we can spawn another meow
    let current = ACTIVE_MEOWS.fetch_add(1, Ordering::SeqCst);
    if current >= MAX_CONCURRENT_MEOWS {
        ACTIVE_MEOWS.fetch_sub(1, Ordering::SeqCst);
        eprintln!("[Audio] Max concurrent meows ({MAX_CONCURRENT_MEOWS}) reached, ignoring");
        return;
    }
    
    thread::spawn(|| {
        struct MeowGuard;
        impl Drop for MeowGuard {
            fn drop(&mut self) {
                ACTIVE_MEOWS.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = MeowGuard;
        
        let stream_handle = OutputStreamBuilder::open_default_stream()
            .expect("Failed to open default audio stream");
        
        let settings = settings::GlobalSettings::load().expect("Failed to load settings");
        let audio_dir = settings.audio_directory.join("audio");
        
        if let Err(e) = ensure_audio_files(&audio_dir) {
            eprintln!("[Audio] Failed to extract bundled audio: {e}");
            return;
        }
        
        let audio_files: Vec<_> = fs::read_dir(&audio_dir)
            .expect("Failed to read audio directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                name.ends_with(".mp3") || name.ends_with(".wav") || 
                name.ends_with(".flac") || name.ends_with(".ogg")
            })
            .collect();
        
        if audio_files.is_empty() {
            eprintln!("[Audio] No audio files found");
            return;
        }
        
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get unix time");
        let mut source = random::default(unix_time.as_micros() as u64);
        let index = source.read_u64() % audio_files.len() as u64;
        
        let audio_file = &audio_files[index as usize];
        
        let handle = fs::File::open(audio_file.path()).expect("Failed to open audio file");
        let file = BufReader::new(handle);
        let sink = rodio::play(&stream_handle.mixer(), file).unwrap();
        sink.sleep_until_end();
    });
}
