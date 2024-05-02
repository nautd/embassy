//! TSC Peripheral Interface

#![macro_use]

/// Enums defined for peripheral parameters
pub mod enums;

use core::marker::PhantomData;

use crate::gpio::{low_level::AFType, AnyPin};
use crate::{pac::tsc::Tsc as Regs, rcc::RccPeripheral};
use crate::{peripherals, Peripheral};
use embassy_hal_internal::{into_ref, PeripheralRef};

pub use enums::*;

const TSC_NUM_GROUPS: u32 = 8;

/// Error type defined for TSC
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Test error for TSC
    Test,
}

/// Pin type definition to control IO parameters
pub enum PinType {
    /// Sensing channel pin connected to an electrode
    Channel,
    /// Sampling capacitor pin, one required for every pin group
    Sample,
    /// Shield pin connected to capacitive sensing shield
    Shield,
}

/// Unclear
pub struct TscGroup {}

/// Peripheral state
#[derive(PartialEq, Clone, Copy)]
pub enum State {
    /// Peripheral is being setup or reconfigured
    Reset,
    /// Ready to start acquisition
    Ready,
    /// In process of sensor acquisition
    Busy,
    /// Error occured during acquisition
    Error,
}

/// Individual group status checked after acquisition reported as complete
/// For groups with multiple channel pins, may take longer because acquisitions
/// are done sequentially. Check this status before pulling count for each
/// sampled channel
#[derive(PartialEq)]
pub enum GroupStatus {
    /// Acquisition for channel still in progress
    Ongoing,
    /// Acquisition either not started or complete
    Complete,
}

/// Group identifier used to interrogate status
#[allow(missing_docs)]
pub enum Group {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
}

impl Into<usize> for Group {
    fn into(self) -> usize {
        match self {
            Group::One => 0,
            Group::Two => 1,
            Group::Three => 2,
            Group::Four => 3,
            Group::Five => 4,
            Group::Six => 5,
            Group::Seven => 6,
            Group::Eight => 7,
        }
    }
}

/// Peripheral configuration
#[derive(Clone, Copy)]
pub struct Config {
    /// Duration of high state of the charge transfer pulse
    pub ct_pulse_high_length: ChargeTransferPulseCycle,
    /// Duration of the low state of the charge transfer pulse
    pub ct_pulse_low_length: ChargeTransferPulseCycle,
    /// Enable/disable of spread spectrum feature
    pub spread_spectrum: bool,
    /// Adds variable number of periods of the SS clk to pulse high state
    pub spread_spectrum_deviation: SSDeviation,
    /// Selects AHB clock divider used to generate SS clk
    pub spread_spectrum_prescaler: bool,
    /// Selects AHB clock divider used to generate pulse generator clk
    pub pulse_generator_prescaler: PGPrescalerDivider,
    /// Maximum number of charge tranfer pulses that can be generated before error
    pub max_count_value: MaxCount,
    /// Defines config of all IOs when no ongoing acquisition
    pub io_default_mode: bool,
    /// Polarity of sync input pin
    pub synchro_pin_polarity: bool,
    /// Acquisition starts when start bit is set or with sync pin input
    pub acquisition_mode: bool,
    /// Enable max count interrupt
    pub max_count_interrupt: bool,
    /// Channel IO mask
    pub channel_ios: u32,
    /// Shield IO mask
    pub shield_ios: u32,
    /// Sampling IO mask
    pub sampling_ios: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ct_pulse_high_length: ChargeTransferPulseCycle::_1,
            ct_pulse_low_length: ChargeTransferPulseCycle::_1,
            spread_spectrum: false,
            spread_spectrum_deviation: SSDeviation::new(1).unwrap(),
            spread_spectrum_prescaler: false,
            pulse_generator_prescaler: PGPrescalerDivider::_1,
            max_count_value: MaxCount::_255,
            io_default_mode: false,
            synchro_pin_polarity: false,
            acquisition_mode: false,
            max_count_interrupt: false,
            channel_ios: 0,
            shield_ios: 0,
            sampling_ios: 0,
        }
    }
}

/// Pin struct that maintains usage
#[allow(missing_docs)]
pub struct TscPin<'d, T, C> {
    _pin: PeripheralRef<'d, AnyPin>,
    _role: PinType,
    phantom: PhantomData<(T, C)>,
}

/// Pin group definition
/// Pins are organized into groups of four IOs, all groups with a
/// sampling channel must also have a sampling capacitor channel.
#[allow(missing_docs)]
#[derive(Default)]
pub struct PinGroup<'d, T, C> {
    d1: Option<TscPin<'d, T, C>>,
    d2: Option<TscPin<'d, T, C>>,
    d3: Option<TscPin<'d, T, C>>,
    d4: Option<TscPin<'d, T, C>>,
}

impl<'d, T: Instance, C> PinGroup<'d, T, C> {
    /// Create new sensing group
    pub fn new() -> Self {
        Self {
            d1: None,
            d2: None,
            d3: None,
            d4: None,
        }
    }
}

macro_rules! group_impl {
    ($group:ident, $trait1:ident, $trait2:ident, $trait3:ident, $trait4:ident) => {
        impl<'d, T: Instance> PinGroup<'d, T, $group> {
            #[doc = concat!("Create a new pin1 for ", stringify!($group), " TSC group instance.")]
            pub fn set_io1(&mut self, pin: impl Peripheral<P = impl $trait1<T>> + 'd, role: PinType) {
                into_ref!(pin);
                critical_section::with(|_| {
                    pin.set_low();
                    pin.set_as_af(
                        pin.af_num(),
                        match role {
                            PinType::Channel => AFType::OutputPushPull,
                            PinType::Sample => AFType::OutputOpenDrain,
                            PinType::Shield => AFType::OutputPushPull,
                        },
                    );
                    self.d1 = Some(TscPin {
                        _pin: pin.map_into(),
                        _role: role,
                        phantom: PhantomData,
                    })
                })
            }

            #[doc = concat!("Create a new pin2 for ", stringify!($group), " TSC group instance.")]
            pub fn set_io2(&mut self, pin: impl Peripheral<P = impl $trait2<T>> + 'd, role: PinType) {
                into_ref!(pin);
                critical_section::with(|_| {
                    pin.set_low();
                    pin.set_as_af(
                        pin.af_num(),
                        match role {
                            PinType::Channel => AFType::OutputPushPull,
                            PinType::Sample => AFType::OutputOpenDrain,
                            PinType::Shield => AFType::OutputPushPull,
                        },
                    );
                    self.d2 = Some(TscPin {
                        _pin: pin.map_into(),
                        _role: role,
                        phantom: PhantomData,
                    })
                })
            }

            #[doc = concat!("Create a new pin3 for ", stringify!($group), " TSC group instance.")]
            pub fn set_io3(&mut self, pin: impl Peripheral<P = impl $trait3<T>> + 'd, role: PinType) {
                into_ref!(pin);
                critical_section::with(|_| {
                    pin.set_low();
                    pin.set_as_af(
                        pin.af_num(),
                        match role {
                            PinType::Channel => AFType::OutputPushPull,
                            PinType::Sample => AFType::OutputOpenDrain,
                            PinType::Shield => AFType::OutputPushPull,
                        },
                    );
                    self.d3 = Some(TscPin {
                        _pin: pin.map_into(),
                        _role: role,
                        phantom: PhantomData,
                    })
                })
            }

            #[doc = concat!("Create a new pin4 for ", stringify!($group), " TSC group instance.")]
            pub fn set_io4(&mut self, pin: impl Peripheral<P = impl $trait4<T>> + 'd, role: PinType) {
                into_ref!(pin);
                critical_section::with(|_| {
                    pin.set_low();
                    pin.set_as_af(
                        pin.af_num(),
                        match role {
                            PinType::Channel => AFType::OutputPushPull,
                            PinType::Sample => AFType::OutputOpenDrain,
                            PinType::Shield => AFType::OutputPushPull,
                        },
                    );
                    self.d4 = Some(TscPin {
                        _pin: pin.map_into(),
                        _role: role,
                        phantom: PhantomData,
                    })
                })
            }
        }
    };
}

group_impl!(G1, G1IO1Pin, G1IO2Pin, G1IO3Pin, G1IO4Pin);
group_impl!(G2, G2IO1Pin, G2IO2Pin, G2IO3Pin, G2IO4Pin);
group_impl!(G3, G3IO1Pin, G3IO2Pin, G3IO3Pin, G3IO4Pin);
group_impl!(G4, G4IO1Pin, G4IO2Pin, G4IO3Pin, G4IO4Pin);
group_impl!(G5, G5IO1Pin, G5IO2Pin, G5IO3Pin, G5IO4Pin);
group_impl!(G6, G6IO1Pin, G6IO2Pin, G6IO3Pin, G6IO4Pin);
group_impl!(G7, G7IO1Pin, G7IO2Pin, G7IO3Pin, G7IO4Pin);
group_impl!(G8, G8IO1Pin, G8IO2Pin, G8IO3Pin, G8IO4Pin);

/// Group 1 marker type.
pub enum G1 {}
/// Group 2 marker type.
pub enum G2 {}
/// Group 3 marker type.
pub enum G3 {}
/// Group 4 marker type.
pub enum G4 {}
/// Group 5 marker type.
pub enum G5 {}
/// Group 6 marker type.
pub enum G6 {}
/// Group 7 marker type.
pub enum G7 {}
/// Group 8 marker type.
pub enum G8 {}

/// TSC driver
pub struct Tsc<'d, T: Instance> {
    _peri: PeripheralRef<'d, T>,
    _g1: Option<PinGroup<'d, T, G1>>,
    _g2: Option<PinGroup<'d, T, G2>>,
    _g3: Option<PinGroup<'d, T, G3>>,
    _g4: Option<PinGroup<'d, T, G4>>,
    _g5: Option<PinGroup<'d, T, G5>>,
    _g6: Option<PinGroup<'d, T, G6>>,
    _g7: Option<PinGroup<'d, T, G7>>,
    _g8: Option<PinGroup<'d, T, G8>>,
    state: State,
    config: Config,
}

impl<'d, T: Instance> Tsc<'d, T> {
    /// Create new TSC driver
    pub fn new(
        peri: impl Peripheral<P = T> + 'd,
        g1: Option<PinGroup<'d, T, G1>>,
        g2: Option<PinGroup<'d, T, G2>>,
        g3: Option<PinGroup<'d, T, G3>>,
        g4: Option<PinGroup<'d, T, G4>>,
        g5: Option<PinGroup<'d, T, G5>>,
        g6: Option<PinGroup<'d, T, G6>>,
        g7: Option<PinGroup<'d, T, G7>>,
        g8: Option<PinGroup<'d, T, G8>>,
        config: Config,
    ) -> Self {
        // Need to check valid pin configuration input
        Self::new_inner(peri, g1, g2, g3, g4, g5, g6, g7, g8, config)
    }

    // fn filter_group() -> Option<PinGroup<'d>> {}
    fn extract_groups(io_mask: u32) -> u32 {
        let mut groups: u32 = 0;
        for idx in 0..TSC_NUM_GROUPS {
            if io_mask & (0x0F << idx * 4) != 0 {
                groups |= 1 << idx
            }
        }
        groups
    }

    fn new_inner(
        peri: impl Peripheral<P = T> + 'd,
        g1: Option<PinGroup<'d, T, G1>>,
        g2: Option<PinGroup<'d, T, G2>>,
        g3: Option<PinGroup<'d, T, G3>>,
        g4: Option<PinGroup<'d, T, G4>>,
        g5: Option<PinGroup<'d, T, G5>>,
        g6: Option<PinGroup<'d, T, G6>>,
        g7: Option<PinGroup<'d, T, G7>>,
        g8: Option<PinGroup<'d, T, G8>>,
        config: Config,
    ) -> Self {
        into_ref!(peri);

        T::enable_and_reset();

        T::REGS.cr().modify(|w| {
            w.set_tsce(true);
            w.set_ctph(config.ct_pulse_high_length.into());
            w.set_ctpl(config.ct_pulse_low_length.into());
            w.set_sse(config.spread_spectrum);
            // Prevent invalid configuration for pulse generator prescaler
            if config.ct_pulse_low_length == ChargeTransferPulseCycle::_1
                && (config.pulse_generator_prescaler == PGPrescalerDivider::_1
                    || config.pulse_generator_prescaler == PGPrescalerDivider::_2)
            {
                w.set_pgpsc(PGPrescalerDivider::_4.into());
            } else if config.ct_pulse_low_length == ChargeTransferPulseCycle::_2
                && config.pulse_generator_prescaler == PGPrescalerDivider::_1
            {
                w.set_pgpsc(PGPrescalerDivider::_2.into());
            } else {
                w.set_pgpsc(config.pulse_generator_prescaler.into());
            }
            w.set_ssd(config.spread_spectrum_deviation.into());
            w.set_sspsc(config.spread_spectrum_prescaler);

            w.set_mcv(config.max_count_value.into());
            w.set_syncpol(config.synchro_pin_polarity);
            w.set_am(config.acquisition_mode);
        });

        // Set IO configuration
        // Disable Schmitt trigger hysteresis on all used TSC IOs
        T::REGS
            .iohcr()
            .write(|w| w.0 = !(config.channel_ios | config.shield_ios | config.sampling_ios));

        // Set channel and shield IOs
        T::REGS.ioccr().write(|w| w.0 = config.channel_ios | config.shield_ios);

        // Set sampling IOs
        T::REGS.ioscr().write(|w| w.0 = config.sampling_ios);

        // Set the groups to be acquired
        T::REGS
            .iogcsr()
            .write(|w| w.0 = Self::extract_groups(config.channel_ios));

        // Disable interrupts
        T::REGS.ier().modify(|w| {
            w.set_eoaie(false);
            w.set_mceie(false);
        });

        // Clear flags
        T::REGS.icr().modify(|w| {
            w.set_eoaic(true);
            w.set_mceic(true);
        });

        Self {
            _peri: peri,
            _g1: g1,
            _g2: g2,
            _g3: g3,
            _g4: g4,
            _g5: g5,
            _g6: g6,
            _g7: g7,
            _g8: g8,
            state: State::Ready,
            config,
        }
    }

    /// Start charge transfer acquisition
    pub fn start(&mut self) {
        self.state = State::Busy;

        // Disable interrupts
        T::REGS.ier().modify(|w| {
            w.set_eoaie(false);
            w.set_mceie(false);
        });

        // Clear flags
        T::REGS.icr().modify(|w| {
            w.set_eoaic(true);
            w.set_mceic(true);
        });

        // Set the touch sensing IOs not acquired to the default mode
        T::REGS.cr().modify(|w| {
            w.set_iodef(self.config.io_default_mode);
        });

        // Start the acquisition
        T::REGS.cr().modify(|w| {
            w.set_start(true);
        });
    }

    /// Start charge transfer acquisition with interrupts enabled
    pub fn start_it(&mut self) {
        self.state = State::Busy;

        // Enable interrupts
        T::REGS.ier().modify(|w| {
            w.set_eoaie(true);
            w.set_mceie(self.config.max_count_interrupt);
        });

        // Clear flags
        T::REGS.icr().modify(|w| {
            w.set_eoaic(true);
            w.set_mceic(true);
        });

        // Set the touch sensing IOs not acquired to the default mode
        T::REGS.cr().modify(|w| {
            w.set_iodef(self.config.io_default_mode);
        });

        // Start the acquisition
        T::REGS.cr().modify(|w| {
            w.set_start(true);
        });
    }

    /// Stop charge transfer acquisition
    pub fn stop(&mut self) {
        T::REGS.cr().modify(|w| {
            w.set_start(false);
        });

        // Set the touch sensing IOs in low power mode
        T::REGS.cr().modify(|w| {
            w.set_iodef(false);
        });

        // Clear flags
        T::REGS.icr().modify(|w| {
            w.set_eoaic(true);
            w.set_mceic(true);
        });

        self.state = State::Ready;
    }

    /// Stop charge transfer acquisition and clear interrupts
    pub fn stop_it(&mut self) {
        T::REGS.cr().modify(|w| {
            w.set_start(false);
        });

        // Set the touch sensing IOs in low power mode
        T::REGS.cr().modify(|w| {
            w.set_iodef(false);
        });

        // Disable interrupts
        T::REGS.ier().modify(|w| {
            w.set_eoaie(false);
            w.set_mceie(false);
        });

        // Clear flags
        T::REGS.icr().modify(|w| {
            w.set_eoaic(true);
            w.set_mceic(true);
        });

        self.state = State::Ready;
    }

    /// Wait for end of acquisition
    pub fn poll_for_acquisition(&mut self) {
        while self.get_state() == State::Busy {}
    }

    /// Get current state of acquisition
    pub fn get_state(&mut self) -> State {
        if self.state == State::Busy {
            if T::REGS.isr().read().eoaf() {
                if T::REGS.isr().read().mcef() {
                    self.state = State::Error
                } else {
                    self.state = State::Ready
                }
            }
        }
        self.state
    }

    /// Get the individual group status to check acquisition complete
    pub fn group_get_status(&mut self, index: Group) -> GroupStatus {
        // Status bits are set by hardware when the acquisition on the corresponding
        // enabled analog IO group is complete, cleared when new acquisition is started
        let status = match index {
            Group::One => T::REGS.iogcsr().read().g1s(),
            Group::Two => T::REGS.iogcsr().read().g2s(),
            Group::Three => T::REGS.iogcsr().read().g3s(),
            Group::Four => T::REGS.iogcsr().read().g4s(),
            Group::Five => T::REGS.iogcsr().read().g5s(),
            Group::Six => T::REGS.iogcsr().read().g6s(),
            Group::Seven => T::REGS.iogcsr().read().g7s(),
            Group::Eight => T::REGS.iogcsr().read().g8s(),
        };
        match status {
            true => GroupStatus::Complete,
            false => GroupStatus::Ongoing,
        }
    }

    /// Get the count for the acquisiton, valid once group status is set
    pub fn group_get_value(&mut self, index: Group) -> u16 {
        T::REGS.iogcr(index.into()).read().cnt()
    }

    // pub fn configure_io()

    /// Discharge the IOs for subsequent acquisition
    pub fn discharge_io(&mut self, status: bool) {
        // Set the touch sensing IOs in low power mode
        T::REGS.cr().modify(|w| {
            w.set_iodef(!status);
        });
    }
}

impl<'d, T: Instance> Drop for Tsc<'d, T> {
    fn drop(&mut self) {
        //  Need to figure out what to do with the IOs
        T::disable();
    }
}

pub(crate) trait SealedInstance {
    const REGS: Regs;
}

/// TSC instance trait
#[allow(private_bounds)]
pub trait Instance: Peripheral<P = Self> + SealedInstance + RccPeripheral {}

foreach_peripheral!(
    (tsc, $inst:ident) => {
        impl SealedInstance for peripherals::$inst {
            const REGS: Regs = crate::pac::$inst;
        }

        impl Instance for peripherals::$inst {}
    };
);

pin_trait!(G1IO1Pin, Instance);
pin_trait!(G1IO2Pin, Instance);
pin_trait!(G1IO3Pin, Instance);
pin_trait!(G1IO4Pin, Instance);
pin_trait!(G2IO1Pin, Instance);
pin_trait!(G2IO2Pin, Instance);
pin_trait!(G2IO3Pin, Instance);
pin_trait!(G2IO4Pin, Instance);
pin_trait!(G3IO1Pin, Instance);
pin_trait!(G3IO2Pin, Instance);
pin_trait!(G3IO3Pin, Instance);
pin_trait!(G3IO4Pin, Instance);
pin_trait!(G4IO1Pin, Instance);
pin_trait!(G4IO2Pin, Instance);
pin_trait!(G4IO3Pin, Instance);
pin_trait!(G4IO4Pin, Instance);
pin_trait!(G5IO1Pin, Instance);
pin_trait!(G5IO2Pin, Instance);
pin_trait!(G5IO3Pin, Instance);
pin_trait!(G5IO4Pin, Instance);
pin_trait!(G6IO1Pin, Instance);
pin_trait!(G6IO2Pin, Instance);
pin_trait!(G6IO3Pin, Instance);
pin_trait!(G6IO4Pin, Instance);
pin_trait!(G7IO1Pin, Instance);
pin_trait!(G7IO2Pin, Instance);
pin_trait!(G7IO3Pin, Instance);
pin_trait!(G7IO4Pin, Instance);
pin_trait!(G8IO1Pin, Instance);
pin_trait!(G8IO2Pin, Instance);
pin_trait!(G8IO3Pin, Instance);
pin_trait!(G8IO4Pin, Instance);
