use crate::message::Message;
use crate::midi::NoteEvent;
use crate::synth::Synth;
use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, SampleFormat,
};
use crossbeam::channel;
use std::thread::{self, JoinHandle};

pub struct AudioManager {
    handle: Option<JoinHandle<()>>,
    shutdown: channel::Sender<()>,
}

impl AudioManager {
    pub fn new(
        midi_events: channel::Receiver<NoteEvent>,
        // TODO callback trait instead of channel
        message_sender: channel::Sender<Message>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = channel::bounded(1);
        // run this in a thread since it causes errors if run before the gui on a thread
        let handle = thread::spawn(move || {
            // emulate try block
            match (|| -> Result<_> {
                let host = cpal::default_host();
                let device = host
                    .default_output_device()
                    .ok_or_else(|| anyhow!("no output audio device found"))?;
                let supported_config = device
                    .supported_output_configs()?
                    .filter(|config| {
                        // only stereo configs
                        config.sample_format() == SampleFormat::F32 && config.channels() == 2
                    })
                    // just pick the first valid config
                    .next()
                    .ok_or_else(|| anyhow!("no valid output audio config found"))?;
                let default_sample_rate = device.default_output_config()?.sample_rate();
                let config = supported_config
                    .with_sample_rate(default_sample_rate)
                    .config();
                let mut synth = Synth::new(config.sample_rate.0, 2);
                let message_sender_clone = message_sender.clone();
                let stream = device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        while let Ok(event) = midi_events.try_recv() {
                            match event {
                                NoteEvent::On { freq, velocity } => {
                                    synth.set_freq(freq);
                                    synth.set_amplitude(velocity);
                                }
                                NoteEvent::Off => {
                                    synth.set_amplitude(0f32);
                                }
                            }
                        }
                        synth.play(data);
                    },
                    move |error| {
                        message_sender_clone
                            .send(Message::Status(format!("error: {:?}", error)))
                            .unwrap();
                    },
                )?;
                message_sender
                    .send(Message::AudioName(device.name().unwrap()))
                    .unwrap();
                // can't return stream since it isn't Send
                stream.play()?;
                // return the stream to keep it alive
                Ok(stream)
            })() {
                Ok(_stream) => {
                    shutdown_rx.recv().unwrap();
                }
                Err(e) => {
                    message_sender
                        .send(Message::Status(format!("error: {}", e)))
                        .unwrap();
                }
            }
        });
        Self {
            handle: Some(handle),
            shutdown: shutdown_tx,
        }
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.shutdown.send(()).unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
