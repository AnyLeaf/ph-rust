//! This program demonstrates how to use the AnyLeaf pH module
//! with an STM32F3 microcontroller. It continuously displays
//! the pH using an output handler, and demonstrates how
//! to calibrate.

#![no_main]
#![no_std]

use cortex_m;
use cortex_m_rt::entry;
use stm32f3xx_hal as hal;
use hal::{delay::Delay, i2c::I2c, prelude::*, stm32};
use embedded_hal::blocking::delay::DelayMs;

use anyleaf::{Rtd, CalPtT, RtdType, RtdWires};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // Set up i2C.
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32::Peripherals::take().unwrap();

    let stim = &mut cp.ITM.stim[0];

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(cp.SYST, clocks);

    le.GPIOB.split(&mut rcc.ahb); // PB GPIO pins

    // Set up SPI, where SCK is on PA5, MISO is on PA6, MOSI is on PA7, CS is on PA8,
    // and RDY is on PA9.
    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);
    let sck = gpioa.pa5.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let miso = gpioa.pa6.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let mosi = gpioa.pa7.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let cs = gpioa
        .pa8
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);

    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    let mut spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        spi_mode,
        4.mhz(),
        clocks,
        &mut rcc.apb2,
    );

    let dt = 1.; // Time between measurements, in seconds
    let mut rtd = Rtd::new(&mut spi, cs, RtdType::Pt100, RtdWires::Three);

    loop {
        rprintln!("Temp: {}", rtd.read().unwrap());

        delay.delay_ms(dt as u16 * 1000);
    }
}

// This handler will cause a crash if present in Debug, and one if not present
// in Release mode. We should be only building in release mode, since it offers
// a large performance boost. So much so, that any increase in compile time
// is offset by the faster init speed.
#[panic_handler]
fn my_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
