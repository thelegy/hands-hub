use std::sync::{Arc, Mutex};

use evdev::{AttributeSet, InputEvent, InputEventKind, Key, RelativeAxisType, Synchronization};
use tokio::task::JoinSet;

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
        Arc::new(Mutex::new(uintput_builder.build().unwrap()))
    };

    for (path, mut device) in devices {
        let path_str = path.display();
        println!("Found path: {path_str}");
        device.grab().unwrap();
        let mut events = device.into_event_stream().unwrap();
        let uinput = uinput.clone();
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
                        println!("Queue event: {:?} {}", event.kind(), event.value());
                        events_buf.push(event);
                    }
                    println!("Emit {} events", events_buf.len());
                    uinput.lock().unwrap().emit(&events_buf).unwrap();
                }
            }
        });
    }
    join_set.join_all().await;
}
