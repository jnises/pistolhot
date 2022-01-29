use crate::timer::Timer;
use anyhow::{anyhow, bail, Result};
use chrono::Duration;
use crossbeam::channel;
use log::{error, warn};
use midir::{MidiInput, MidiInputConnection};
use std::{convert::TryFrom, sync::{Arc, Mutex}};
use wmidi::MidiMessage;

type MidiSender = channel::Sender<MidiMessage<'static>>;

pub struct MidiReader {
    midi_events: MidiSender,
    timer: Timer,
    port: Mutex<Option<(MidiInputConnection<()>, String)>>,
}

impl MidiReader {
    pub fn new(midi_events: MidiSender) -> Arc<Self> {
        let aself = Arc::new(Self {
            timer: Timer::new(),
            port: Mutex::new(None),
            midi_events,
        });
        aself.init();
        aself
    }

    fn init(self: &Arc<Self>) {
        debug_assert!(self.port.lock().unwrap().is_none());
        let r = (|| -> Result<()> {
            let midi = MidiInput::new("wayfarer")?;
            let ports = midi.ports();
            Ok(if let Some(port) = ports.first() {
                let name = midi.port_name(port)?;
                let midi_events = self.midi_events.clone();
                let connection = midi
                    .connect(
                        port,
                        &name,
                        move |_time_ms, message, _| match wmidi::MidiMessage::try_from(message) {
                            Ok(message) => {
                                if let Err(e) = midi_events.try_send(message.to_owned()) {
                                    error!("error sending midi event {}", e);
                                }
                            }
                            Err(e) => {
                                error!("error parsing midi event {}", e);
                            }
                        },
                        (),
                    )
                    .map_err(|e| anyhow!("{}", e))?;
                *self.port.lock().unwrap() = Some((connection, name));
            } else {
                bail!("no midi so far");
            })
        })();
        if let Err(e) = r {
            warn!("error setting up midi: {}. retrying", e);
            let weak_self = Arc::downgrade(&self);
            self.timer
                .schedule_with_delay(&Duration::seconds(1), move || {
                    if let Some(s) = weak_self.upgrade() {
                        s.init();
                    }
                });
        }
    }

    pub fn get_name(&self) -> String {
        self.port.lock().unwrap().as_ref().map(|(_, name)| name.clone()).unwrap_or("-".to_string())
    }
}
