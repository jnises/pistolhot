use synth::SynthPlayer;
use vst::{
    plugin::{Category, HostCallback, Info, Plugin},
    plugin_main,
};

struct Data {
    sample_rate: u32,
    synth: synth::Synth,
    midi_sender: crossbeam::channel::Sender<wmidi::MidiMessage<'static>>,
}

struct PistolhotVst(Option<Data>);

impl PistolhotVst {
    /// panics if default constructed
    fn get_mut_data(&mut self) -> &mut Data {
        self.0.as_mut().unwrap()
    }
}

fn init_logging() {
    log_panics::init();
    use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Logger, Naming};
    let log_folder = dirs::data_local_dir()
        .unwrap()
        .join("org.deepness.pistolhot")
        .join("logs");
    Logger::try_with_str("warning")
        .unwrap()
        .log_to_file(FileSpec::default().directory(log_folder))
        .rotate(
            Criterion::Age(Age::Day), // - create a new file every day
            Naming::Timestamps,       // - let the rotated files have a timestamp in their name
            Cleanup::KeepLogFiles(7), // - keep at most 7 log files
        )
        .start()
        .unwrap();
}

impl Plugin for PistolhotVst {
    fn new(_host: HostCallback) -> Self {
        init_logging();

        let (midi_sender, midi_receiver) = crossbeam::channel::bounded(1024);
        let synth = synth::Synth::new(midi_receiver);
        let sample_rate = 44100;
        Self(Some(Data {
            sample_rate,
            synth,
            midi_sender,
        }))
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Pistolhot".to_string(),
            unique_id: 1073986287, // Used by hosts to differentiate between plugins.
            category: Category::Synth,
            inputs: 0,
            outputs: 2,
            ..Default::default()
        }
    }

    fn can_do(&self, can_do: vst::plugin::CanDo) -> vst::api::Supported {
        match can_do {
            vst::plugin::CanDo::ReceiveEvents => vst::api::Supported::Yes,
            vst::plugin::CanDo::ReceiveMidiEvent => vst::api::Supported::Yes,
            _ => vst::api::Supported::No,
        }
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.get_mut_data().sample_rate = rate as u32;
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        let sender = &mut self.get_mut_data().midi_sender;
        for e in events.events() {
            match e {
                vst::event::Event::Midi(me) => {
                    // TODO don't unwrap. log
                    if let Some(m) = wmidi::MidiMessage::try_from(&me.data[..])
                        .unwrap()
                        .drop_unowned_sysex()
                    {
                        sender.send(m).unwrap();
                    }
                }
                _ => {}
            }
        }
    }

    fn process(&mut self, buffer: &mut vst::buffer::AudioBuffer<f32>) {
        let data = self.get_mut_data();
        // TODO keep scratch buffer to avoid allocations, or change the synthplayer trait to handle non-interleaved channels
        let num_samples = buffer.samples();
        let (_, mut outputs) = buffer.split();
        let channels = outputs.len();
        let mut interleaved = vec![0f32; channels * num_samples];
        data.synth
            .play(data.sample_rate, outputs.len(), interleaved.as_mut_slice());
        for (channel, buf) in outputs.into_iter().enumerate() {
            debug_assert!(buf.len() == num_samples);
            for (sampleidx, b) in buf.iter_mut().enumerate() {
                *b = interleaved[channels * sampleidx + channel];
            }
        }
    }
}

impl Default for PistolhotVst {
    fn default() -> Self {
        Self(None)
    }
}

plugin_main!(PistolhotVst);
