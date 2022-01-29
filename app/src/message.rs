use wmidi::MidiMessage;

pub enum Message {
    Midi(MidiMessage<'static>),
    SetDistorsion(f32),
}

impl From<MidiMessage<'static>> for Message {
    fn from(msg: MidiMessage<'static>) -> Self {
        Message::Midi(msg)
    }
}