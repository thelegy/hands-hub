use std::{
    collections::HashMap,
    hash::Hash,
    io::Write,
    sync::{Arc, Mutex},
};

use evdev::{
    uinput::VirtualDevice, AttributeSet, InputEvent, InputEventKind, Key, RelativeAxisType,
    Synchronization,
};
use tokio::task::JoinSet;

pub trait Sink {
    fn emit_events(&mut self, events: &[InputEvent]);
}

impl Sink for VirtualDevice {
    fn emit_events(&mut self, events: &[InputEvent]) {
        self.emit(events).unwrap()
    }
}

pub struct VoidSink;
impl Sink for VoidSink {
    fn emit_events(&mut self, _events: &[InputEvent]) {}
}

pub struct SinkSelector<K>
where
    K: Eq + Hash + Send,
{
    sinks: HashMap<K, Box<dyn Sink + Send>>,
    pub active: Option<K>,
}
impl<K> SinkSelector<K>
where
    K: Eq + Hash + Send,
{
    pub fn new() -> SinkSelector<K> {
        SinkSelector {
            sinks: Default::default(),
            active: None,
        }
    }
    pub fn add_sink(&mut self, key: K, sink: impl Sink + Send + 'static) {
        let sink = Box::new(sink) as Box<dyn Sink + Send>;
        if let Some(_old_sink) = self.sinks.insert(key, sink) {
            todo!()
        }
    }
    pub fn activate(&mut self, key: K) {
        self.active = Some(key);
    }
}
impl<K> Default for SinkSelector<K>
where
    K: Eq + Hash + Send,
{
    fn default() -> Self {
        Self::new()
    }
}
impl<K> Sink for SinkSelector<K>
where
    K: Eq + Hash + Send,
{
    fn emit_events(&mut self, events: &[InputEvent]) {
        if let Some(active_key) = &self.active {
            if let Some(active_sink) = self.sinks.get_mut(active_key) {
                active_sink.emit_events(events)
            }
        }
    }
}

impl Sink for tokio_serial::SerialStream {
    fn emit_events(&mut self, events: &[InputEvent]) {
        for event in events {
            let bytes = postcard::to_stdvec_cobs(&input_events::InputEvent::from(event)).unwrap();
            self.write_all(&bytes).unwrap();
        }
        let bytes = postcard::to_stdvec_cobs(&input_events::InputEvent::SYN_REPORT).unwrap();
        self.write_all(&bytes).unwrap();
    }
}

#[tokio::main]
async fn main() {
    let mut join_set = JoinSet::new();

    let devices = evdev::enumerate();
    //let path = "/dev/input/by-id/usb-Logitech_HID_compliant_keyboard-event-kbd";
    //let devices = [(PathBuf::from(path), Device::open(path).unwrap())];

    let uinput = {
        let mut keys = AttributeSet::new();
        for i in 0..0x300 {
            keys.insert(Key::new(i));
        }
        let mut rel_axis = AttributeSet::new();
        rel_axis.insert(RelativeAxisType::REL_X);
        rel_axis.insert(RelativeAxisType::REL_Y);
        rel_axis.insert(RelativeAxisType::REL_Z);
        rel_axis.insert(RelativeAxisType::REL_RX);
        rel_axis.insert(RelativeAxisType::REL_RY);
        rel_axis.insert(RelativeAxisType::REL_RZ);
        rel_axis.insert(RelativeAxisType::REL_HWHEEL);
        rel_axis.insert(RelativeAxisType::REL_DIAL);
        rel_axis.insert(RelativeAxisType::REL_WHEEL);
        rel_axis.insert(RelativeAxisType::REL_MISC);
        rel_axis.insert(RelativeAxisType::REL_RESERVED);
        rel_axis.insert(RelativeAxisType::REL_WHEEL_HI_RES);
        rel_axis.insert(RelativeAxisType::REL_HWHEEL_HI_RES);
        let uintput_builder = evdev::uinput::VirtualDeviceBuilder::new()
            .unwrap()
            .name("hands-hub")
            .with_keys(&keys)
            .unwrap()
            .with_relative_axes(&rel_axis)
            .unwrap();
        uintput_builder.build().unwrap()
    };

    let mut sink_selector = SinkSelector::new();
    sink_selector.add_sink("local", uinput);
    sink_selector.activate("local");

    let serial = {
        let builder = tokio_serial::new("/dev/ttyACM1", 0);
        tokio_serial::SerialStream::open(&builder).unwrap()
    };

    sink_selector.add_sink("usb", serial);

    let sink_selector = Arc::new(Mutex::new(sink_selector));

    for (path, mut device) in devices {
        let path_str = path.display();
        println!("Found path: {path_str}");
        if let Some(name) = path.file_name() {
            match &name.to_str() {
                Some("event18") => continue,
                Some("event19") => continue,
                Some("mouse1") => continue,
                _ => {}
            }
        }
        device.grab().unwrap();
        let mut events = device.into_event_stream().unwrap();
        let sink_selector = sink_selector.clone();
        join_set.spawn(async move {
            loop {
                let mut events_buf: Vec<InputEvent> = vec![];
                loop {
                    let event = events.next_event().await.unwrap();
                    if let InputEventKind::Synchronization(Synchronization::SYN_REPORT) =
                        event.kind()
                    {
                        break;
                    } else {
                        if let InputEventKind::Key(Key::KEY_F15) = event.kind() {
                            if event.value() == 1 {
                                // keydown
                                let mut sink_selector = sink_selector.lock().unwrap();
                                if sink_selector.active != Some("local") {
                                    sink_selector.activate("local")
                                } else {
                                    sink_selector.activate("usb")
                                }
                            }
                            break;
                        }
                        println!("Queue event: {:?} {}", event.kind(), event.value());
                        events_buf.push(event);
                    }
                    println!("Emit {} events", events_buf.len());
                    sink_selector.lock().unwrap().emit_events(&events_buf);
                }
            }
        });
    }
    join_set.join_all().await;
}
