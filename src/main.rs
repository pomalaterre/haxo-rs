extern crate fluidsynth;
extern crate rand;
extern crate time;

#[macro_use]
extern crate static_assertions;

use log::{debug, /* error, info, */ warn};

use fluidsynth::{audio, settings, synth};
use std::error::Error;
use std::process::Command;
use std::thread;
use std::time::Duration;

mod keyscan;
mod notemap;

fn try_init_synth() -> (synth::Synth, settings::Settings, audio::AudioDriver) {
    let mut settings = settings::Settings::new();
    // try to optimize for low latency
    if !settings.setstr("audio.driver", "alsa") {
        warn!("Setting audio.driver in fluidsynth failed");
    }
    if !settings.setint("audio.periods", 3) {
        warn!("Setting audio.periods in fluidsynth failed");
    }
    if !settings.setint("audio.period-size", 444) {
        warn!("Setting audio.period-size in fluidsynth failed");
    }
    // TODO: Find headphone device, as it may not always be hw:1
    // if HDMI is disabled
    if !settings.setstr("audio.alsa.device", "hw:1") {
        warn!("Setting audio.alsa.device in fluidsynth failed");
    }
    if !settings.setint("audio.realtime-prio", 99) {
        warn!("Setting audio.realtime-prio in fluidsynth failed");
    }
    let mut syn = synth::Synth::new(&mut settings);
    // supposedly, assign tenor sax patch to midi channel 0
    syn.program_change(0, 67);
    if !syn.set_polyphony(1) {
        warn!("Failed to set polyphony to 1");
    }
    let adriver = audio::AudioDriver::new(&mut settings, &mut syn);
    //syn.sfload("/usr/share/sounds/sf2/FluidR3_GM.sf2", 1);
    syn.sfload("/usr/share/sounds/sf2/TimGM6mb.sf2", 1);
    println!("Synth created");
    (syn, settings, adriver)
}

fn shutdown() {
    debug!("Bye...");
    Command::new("/usr/bin/sudo")
        .arg("/usr/sbin/halt")
        .status()
        .expect("failed to halt system");
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let (syn, _settings, _adriver) = try_init_synth();

    println!("Starting haxophone...");

    keyscan::init_io().expect("Failed to initialize scan GPIO");

    let notemap = notemap::generate();

    let mut last_keys: u32 = 0;
    let mut last_note = 0;
    loop {
        thread::sleep(Duration::from_millis(10));

        let keys = keyscan::scan()?;
        if last_keys != keys {
            debug!("Key event {:032b}: {}", keys, keys);
            keyscan::debug_print(keys);
            if let Some(note) = notemap.get(&keys) {
                // until we have breadth control, assume all keys unpressed means silence
                if *note > 0 {
                    syn.noteon(0, *note, 127);
                }
                if *note < 0 {
                    // TODO: pick the right control messages.  For now, only one is supported
                    shutdown();
                    return Ok(());
                }
                // make before break
                syn.noteoff(0, last_note);
                last_note = *note;
                debug!("last_note changed to {}", last_note);
            }
            last_keys = keys;
        }
    }
}
