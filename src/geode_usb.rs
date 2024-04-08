//! Handle all USB communcation in this task.
//! If USB is handled in multiple tasks the code gets weird and unwieldy (`'static` everywhere)
//! Code in this file is mostly from the examples folder in embassy-rs.

use embassy_futures::join::join;
use embassy_rp::{peripherals::USB, usb::Driver};

use crate::geode_midi::midi_session;
use crate::geode_midi;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::class::cdc_acm::State;
use embassy_usb::class::midi::MidiClass;
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};

#[embassy_executor::task]
pub async fn usb_task(
    // remember this is the Driver struct not the trait
    driver: Driver<'static, USB>
) {
    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("dogeystamp");
    config.product = Some("Geode-Piano MIDI keyboard");
    config.serial_number = Some("alpha-12345");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut device_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut logger_state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut midi_class = MidiClass::new(&mut builder, 1, 1, 64);
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);
    let log_fut = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, logger_class);

    // The `MidiClass` can be split into `Sender` and `Receiver`, to be used in separate tasks.
    // let (sender, receiver) = class.split();

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let midi_fut = async {
        loop {
            log::info!("Connected");
            midi_session(&mut midi_class).await;
            log::info!("Disconnected");
        }
    };

    join(usb_fut, join(log_fut, midi_fut)).await;
}
