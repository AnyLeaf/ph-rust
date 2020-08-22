//! Code that triggers parts of the ec circuit: Analog Devices CN-0411.

use embedded_hal::{
    adc::OneShot,
    blocking::{
        delay::{DelayMs, DelayUs},
        i2c, spi,
    },
    digital::v2::OutputPin,
    PwmPin,
};

use crate::mcp4921::{self, Mcp4921};
use ads1x1x::{self, channel::{SingleA2, SingleA3}};

// Frequencies for the PWM channels.
const F_LOW: u16 = 94; // uS range
const F_HIGH: u16 = 2_400; // mS range

#[derive(Clone, Copy, Debug)]
pub enum EcGain {
    // Gain 1 isn't used. We set it up this way
    // so the gains match with the S values.
    Two,   // S2
    Three, // S3
    Four,  // S4
    Five,  // S5
    Six,   // S6
    Seven, // S7
    Eight, // S8
}

impl EcGain {
    /// Raise by one level.
    fn raise(&self) -> Self {
        match self {
            Self::Two => Self::Three,
            Self::Three => Self::Four,
            Self::Four => Self::Five,
            Self::Five => Self::Six,
            Self::Six => Self::Seven,
            Self::Seven => Self::Eight,
            Self::Eight => panic!("Gain is already at maximum"),
        }
    }
    /// Drop by one level.
    fn drop(&self) -> Self {
        match self {
            Self::Two => panic!("Gain is already at minimum"),
            Self::Three => Self::Two,
            Self::Four => Self::Three,
            Self::Five => Self::Four,
            Self::Six => Self::Five,
            Self::Seven => Self::Six,
            Self::Eight => Self::Seven,
        }
    }
}

/// Use the ADG1608 to select the right resistor.
#[derive(Debug)]
struct ADG1608<P0: OutputPin, P1: OutputPin, P2: OutputPin> {
    pin0: P0,
    pin1: P1,
    pin2: P2,
}

impl<P0: OutputPin, P1: OutputPin, P2: OutputPin> ADG1608<P0, P1, P2> {
    pub fn new(pin0: P0, pin1: P1, pin2: P2) -> Self {
        Self { pin0, pin1, pin2 }
    }

    fn set(&mut self, gain: EcGain) {
        // Enable pin must be pulled high for this to work.
        match gain {
            EcGain::Two => {
                self.pin0.set_high().ok();
                self.pin1.set_low().ok();
                self.pin2.set_low().ok();
            }
            EcGain::Three => {
                self.pin0.set_low().ok();
                self.pin1.set_high().ok();
                self.pin2.set_low().ok();
            }
            EcGain::Four => {
                self.pin0.set_high().ok();
                self.pin1.set_high().ok();
                self.pin2.set_low().ok();
            }
            EcGain::Five => {
                self.pin0.set_low().ok();
                self.pin1.set_low().ok();
                self.pin2.set_high().ok();
            }
            EcGain::Six => {
                self.pin0.set_high().ok();
                self.pin1.set_low().ok();
                self.pin2.set_high().ok();
            }
            EcGain::Seven => {
                self.pin0.set_low().ok();
                self.pin1.set_high().ok();
                self.pin2.set_high().ok();
            }
            EcGain::Eight => {
                self.pin0.set_high().ok();
                self.pin1.set_high().ok();
                self.pin2.set_high().ok();
            }
        }
    }
}

/// Configure and start the 3 PWM pulse sets driving the square wave.
pub fn start_pwm<PWM0, PWM1, PWM2, D>(p0: &mut PWM0, p1: &mut PWM1, p2: &mut PWM2, delay: &mut D)
where
    PWM0: PwmPin,
    PWM1: PwmPin,
    PWM2: PwmPin,
    D: DelayUs<u16>,
{
    // todo: embedded hal API is changing with 1.0. Moving to try_enable etc,
    // todo and own pwm module.
    // p0.try_enable().unwrap();
    // p1.try_enable().unwrap();
    // p2.try_enable().unwrap();
    //

    // Yes; they work internally via a counter that triggers different actions as it gets to different values.
    // You get to pick which clock it counts based on,
    // pre-scale the clock, set the compare numbers, put it in various different modes, etc.
    // Timer peripherals can get pretty complicated, even the simple ones, but they enable a lot of really great
    // stuff when you get familiar with how they work.
    // They typically also have "capture" modes where the counter free-runs and some event triggers
    // the current counter value to be saved.
    // pinealservo
    // I think you could do all 3 waveforms with the same center-aligned PWM frequency and start
    // time, and just have PWM2's polarity inverted; PWM2 looks like the inverse duty cycle and
    // polarity from PWM1 if you view them with the same center point.

    p0.enable();
    p1.enable();

    // todo: Is the PWM2 +VCC and 0, or 0 and -VV

    // 94hz: 0.1063829
    // /2: 5319.1489
    delay.delay_us(5_319);
    // TODO: high freq too
    p2.enable();
}

/// Stop all 3 PWM channels
pub fn stop_pwm<PWM0, PWM1, PWM2>(p0: &mut PWM0, p1: &mut PWM1, p2: &mut PWM2)
where
    PWM0: PwmPin,
    PWM1: PwmPin,
    PWM2: PwmPin,
{
    // p0.try_disable().unwrap(); // todo: For embedded_hal v1.0
    // p1.try_disable().unwrap();
    // p2.try_disable().unwrap();

    p0.disable();
    p1.disable();
    p2.disable();
}

/// Set the excitation voltage to a specified value using the DAC.
pub fn set_v_exc<CS, SPI, E>(spi: &mut SPI, dac: &mut Mcp4921<CS>, v: u16)
where
    SPI: spi::Write<u8, Error = E>,
    CS: OutputPin,
{
    // v is in mV
    let cmd = mcp4921::Command::default();
    // todo: Test this.
    let vref = 2.048;
    let digital = (4_095 * v) as f32 / vref; // 12 bit adc.

    dac.send(spi, cmd.value(digital as u16)).ok();
}

/// The high-level struct representing the EC circuit.
#[derive(Debug)]
pub struct EcSensor<CS, P0, P1, P2, PWM0, PWM1, PWM2>
where
    // todo: There has ato be a way to keep these trait bounds local.
    CS: OutputPin, // for the DAC.
    P0: OutputPin,
    P1: OutputPin,
    P2: OutputPin,
    PWM0: PwmPin,
    PWM1: PwmPin,
    PWM2: PwmPin,
{
    dac: Mcp4921<CS>,
    gain_switch: ADG1608<P0, P1, P2>,
    pwm: (PWM0, PWM1, PWM2),
}

impl<CS, P0, P1, P2, PWM0, PWM1, PWM2> EcSensor<CS, P0, P1, P2, PWM0, PWM1, PWM2>
where
    CS: OutputPin,
    P0: OutputPin,
    P1: OutputPin,
    P2: OutputPin,
    PWM0: PwmPin,
    PWM1: PwmPin,
    PWM2: PwmPin,
{
    pub fn new(cs_dac: CS, switch_pins: (P0, P1, P2), mut pwm: (PWM0, PWM1, PWM2)) -> Self {
        // PWM pins must be already configured with frequency of 94Hz.

        // todo: Setting duty temporarily moved back to main fn. Put here once
        // todo you sort out how to get a value from `get_max_duty()`.
        // pwm.0.set_duty(pwm.0.get_max_duty() / 2.); // 50% duty cyle  // todo verify this is right
        // pwm.1.set_duty(pwm.1.get_max_duty() / 6.); // 17% duty cyle
        // pwm.2.set_duty(pwm.2.get_max_duty() / 6.); // 17% duty cyle

        Self {
            dac: Mcp4921::new(cs_dac),
            gain_switch: ADG1608::new(switch_pins.0, switch_pins.1, switch_pins.2),
            pwm,
        }
    }

    /// Set gain and excitation voltage, as an auto-ranging procedure. See CN-0411: Table 12.
    fn set_range<SPI, E, I2C, CsRtd, EI>(
        &mut self,
        spi: &mut SPI,
        sensors: &mut anyleaf::WaterMonitor<I2C, CsRtd, EI>,
    ) -> Result<(), EI> where
        SPI: spi::Write<u8, Error = E>,
        I2C: i2c::WriteRead<Error = EI> + i2c::Write<Error = EI> + i2c::Read<Error = EI>,
        CsRtd: OutputPin,
    {
        // Set multiplexer to highest gain resistance
        let mut gain = EcGain::Eight;
        self.gain_switch.set(gain);

        // Set DAC to Output V_EXC = 400mV
        let mut v_exc = 400;
        set_v_exc(spi, &mut self.dac, v_exc);

        // Read ADC Input V+ and V-
        let (v_p, v_m) = self.read_voltage(sensors)?;

        let v_def = 0.1; // todo: Figure out what this means. Check the AD community thread.

        while v_p + v_m <= 0.3 * 2. * v_exc as f32 {
            gain = gain.drop();

            // todo: DRY!
            // Read ADC Input V+ and V-
            let readings = self.read_voltage(sensors)?;
            v_p = readings.0;
            v_m = readings.1;
        }

        // todo: Mind V vs mV
        v_exc = (v_def * (v_exc as f32) / (v_p + v_m)) as u16;
        set_v_exc(spi, &mut self.dac, v_exc);

        Ok(())
    }

    // Read the two voltages associated with ec measurement from the ADC.
    fn read_voltage<I2C, CsRtd, EI>(
        &self,
        adc: &mut ads1x1x::interface::I2cInterface<I2C>,
    ) -> Result<(f32, f32), EI>
    where
        I2C: i2c::WriteRead<Error = EI> + i2c::Write<Error = EI> + i2c::Read<Error = EI>,
        CsRtd: OutputPin,
    {
        // We use two additional pins on the same ADC as the ORP sensor.
        let v_p =
            block!(adc
                .as_mut()
                .expect("Measurement after I2C freed")
                .read(&mut SingleA2))?
        );

        let v_m = anyleaf::voltage_from_adc(
            block!(adc
                .as_mut()
                .expect("Measurement after I2C freed")
                .read(&mut SingleA3))?,
        );

        Ok((v_p, v_m))
    }

    /// Take a conductivity measurement. Result is in uS/cm
    /// todo: Way to read TDS (sep fn, or perhaps an enum)
    /// todo: Return result or option.
    pub fn read<SPI, D, I2C, CsRtd, EI, E>(
        &mut self,
        spi: &mut SPI,
        adc: &mut ads1x1x::interface::I2cInterface<I2C>,
        delay: &mut D,
    ) -> Result<f32, EI>
    where
        SPI: spi::Write<u8, Error = E>,
        D: DelayUs<u16> + DelayMs<u16>,
        I2C: i2c::WriteRead<Error = EI> + i2c::Write<Error = EI> + i2c::Read<Error = EI>,
        CsRtd: OutputPin,
    {
        // Wake up the DAC from its low-power shutdown mode.
        let cmd = mcp4921::Command::default();
        let cmd = cmd.channel(mcp4921::Channel::Ch0).enable();
        self.dac.send(spi, cmd).ok();

        start_pwm(&mut self.pwm.0, &mut self.pwm.1, &mut self.pwm.2, delay);
        self.set_range(spi, sensors);

        delay.delay_ms(200); // todo experiment

        let (v_p, v_m) = self.read_voltage(adc)?;

        stop_pwm(&mut self.pwm.0, &mut self.pwm.1, &mut self.pwm.2);

        // Set the DAC to use its low-power shutdown mode.
        let cmd = mcp4921::Command::default();
        let cmd = cmd.channel(mcp4921::Channel::Ch0).shutdown();
        self.dac.send(spi, cmd).ok();

        // todo: Turn on/set and turn off DAC.
        Ok(69.)
    }
}
