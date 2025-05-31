#![no_std]
#![no_main]
#![allow(static_mut_refs)]

use embedded_hal::digital::InputPin;
use embedded_hal::digital::OutputPin;
use fugit::ExtU32;
use panic_halt as _;
use rp_pico::entry;
use rp_pico::hal;
use rp_pico::hal::gpio;
use rp_pico::hal::pac;
use rp_pico::hal::pac::interrupt;
use rp_pico::hal::prelude::*;
use rp_pico::hal::timer::Alarm;
use rp_pico::hal::timer::Instant;
use rp_pico::hal::Timer;
use usb_device::{class_prelude::*, prelude::*};
use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::MouseReport;
use usbd_hid::hid_class::HIDClass;

static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;
static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;
static mut USB_HID: Option<HIDClass<hal::usb::UsbBus>> = None;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    #[cfg(feature = "rp2040-e5")]
    {
        let sio = hal::Sio::new(pac.SIO);
        let _pins = rp_pico::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );
    }
    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    unsafe {
        USB_BUS = Some(usb_bus);
    }

    let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    let usb_hid = HIDClass::new(bus_ref, MouseReport::desc(), 60);
    unsafe {
        USB_HID = Some(usb_hid);
    }

    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x17ef, 0x6019))
        .max_power(100)
        .unwrap()
        .usb_rev(usb_device::device::UsbRev::Usb200)
        .device_release(0x6300)
        .strings(&[StringDescriptors::default()
            .manufacturer("Logitech")
            .product("Lenovo USB Optical Mouse")])
        .unwrap()
        .device_class(0)
        .build();
    unsafe {
        USB_DEVICE = Some(usb_dev);
    }

    unsafe {
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
    };

    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    loop {
        let _ = led_pin.set_high();

        let rep_left = MouseReport {
            x: -5,
            y: 0,
            buttons: 0,
            wheel: 0,
            pan: 0,
        };
        push_mouse_movement(rep_left).ok().unwrap_or(0);

        delay.delay_ms(500);

        let rep_right = MouseReport {
            x: 5,
            y: 0,
            buttons: 0,
            wheel: 0,
            pan: 0,
        };
        push_mouse_movement(rep_right).ok().unwrap_or(0);

        delay.delay_ms(500);

        let rep_up = MouseReport {
            x: 0,
            y: -5,
            buttons: 0,
            wheel: 0,
            pan: 0,
        };
        push_mouse_movement(rep_up).ok().unwrap_or(0);

        delay.delay_ms(500);

        let rep_down = MouseReport {
            x: 0,
            y: 5,
            buttons: 0,
            wheel: 0,
            pan: 0,
        };
        push_mouse_movement(rep_down).ok().unwrap_or(0);

        delay.delay_ms(500);
    }
}

fn push_mouse_movement(report: MouseReport) -> Result<usize, usb_device::UsbError> {
    critical_section::with(|_| unsafe { USB_HID.as_mut().map(|hid| hid.push_input(&report)) })
        .unwrap()
}

#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    // Handle USB request
    let usb_dev = USB_DEVICE.as_mut().unwrap();
    let usb_hid = USB_HID.as_mut().unwrap();
    usb_dev.poll(&mut [usb_hid]);
}

// End of file
