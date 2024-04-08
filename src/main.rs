#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio;
use embassy_rp::gpio::AnyPin;
use embassy_rp::gpio::Input;
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Instance;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::class::cdc_acm::State;
use embassy_usb::class::midi::MidiClass;
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn blink_task(pin: embassy_rp::gpio::AnyPin) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        log::info!("led on from task!");
        led.set_high();
        Timer::after_millis(100).await;

        log::info!("led off!");
        led.set_low();
        Timer::after_secs(5).await;
    }
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn button<'d, T: Instance + 'd>(pin: AnyPin, midi: &mut MidiClass<'d, Driver<'d, T>>) -> Result<(), Disconnected> {
    let mut button = Input::new(pin, Pull::Up);
    loop {
        let mut counter = 10;
        button.wait_for_falling_edge().await;
        loop {
            Timer::after_millis(5).await;
            if button.is_low() {
                counter -= 1;
            } else {
                counter = 10;
            }
            if counter <= 0 {
                break;
            }
        }
        log::info!("button press");
        let note_on = [9, 0x90, 72, 64];
        midi.write_packet(&note_on).await?;
        counter = 10;
        button.wait_for_rising_edge().await;
        loop {
            Timer::after_millis(5).await;
            if button.is_high() {
                counter -= 1;
            } else {
                counter = 10;
            }
            if counter <= 0 {
                break;
            }
        }
        log::info!("button release");
        let note_on = [9, 0x80, 72, 0];
        midi.write_packet(&note_on).await?;
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-MIDI example");
    config.serial_number = Some("12345678");
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
    let log_fut = embassy_usb_logger::with_class!(1024, log::LevelFilter::Debug, logger_class);

    // The `MidiClass` can be split into `Sender` and `Receiver`, to be used in separate tasks.
    // let (sender, receiver) = class.split();

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let midi_fut = async {
        midi_class.wait_connection().await;
        log::info!("Connected");
        let _ = button(p.PIN_16.into(), &mut midi_class).await;
        // let _ = midi_echo(&mut midi_class).await;
        log::info!("Disconnected");
    };

    _spawner.spawn(blink_task(p.PIN_25.into())).unwrap();

    join(usb_fut, join(log_fut, midi_fut)).await;
}

async fn midi_echo<'d, T: Instance + 'd>(class: &mut MidiClass<'d, Driver<'d, T>>) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        log::info!("data: {:#?}", data);
        class.write_packet(data).await?;
    }
}
