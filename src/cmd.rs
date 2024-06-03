use std::{thread::sleep, time::Duration};

use rppal::spi;

use crate::{
    draw::{Color, Drawable},
    SpiDevice, SCREEN_HEIGHT, SCREEN_WIDTH,
};

fn to_bit(f: bool, bit: u8) -> u8 {
    if f {
        1 << bit
    } else {
        0
    }
}

pub trait Command {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()>;
}

pub struct PanelSetting {
    // line order (down / up)
    pub ud: bool,
    // shift dir (left / right)
    pub shl: bool,
    // DC-DC converter (off / on)
    pub shd_n: bool,
    // reset (yes / no)
    pub rst_n: bool,
}
pub struct InternalPower;
pub struct PowerOffSequence;
pub struct BoosterSoftStart;
pub struct PLLControl;
pub struct TempSensor;
pub struct VCOMDataInterval {
    pub border_output: Color,
}
pub struct Unknown6022;
pub struct SetResolution;
pub struct UnknownE3AA;
pub struct Draw<'a, T: Drawable>(pub &'a T);

pub struct PowerOn;
pub struct DisplayRefresh;
pub struct PowerOff;

pub struct Init;

impl Command for Init {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        // init
        PanelSetting::default().send(to)?;
        InternalPower.send(to)?;
        PowerOffSequence.send(to)?;
        BoosterSoftStart.send(to)?;
        PLLControl.send(to)?;
        TempSensor.send(to)?;
        VCOMDataInterval {
            border_output: Color::Black,
        }
        .send(to)?;
        Unknown6022.send(to)?;
        SetResolution.send(to)?;
        UnknownE3AA.send(to)?;
        sleep(Duration::from_millis(100));
        VCOMDataInterval {
            border_output: Color::Black,
        }
        .send(to)?;
        Ok(())
    }
}

impl Command for PowerOff {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x02)?;
        to.wait_busy_low();
        Ok(())
    }
}

impl Command for DisplayRefresh {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x12)?;
        to.wait_busy_high();
        Ok(())
    }
}

impl Command for PowerOn {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x04)?;
        to.wait_busy_high();
        Ok(())
    }
}

impl<D: Drawable> Command for Draw<'_, D> {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        SetResolution.send(to)?;
        // each byte fits 2 px
        to.send_cmd(0x10)?;
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH / 2 {
                let c1 = self.0.get_pixel(x * 2, y) as u8;
                let c2 = self.0.get_pixel(x * 2 + 1, y) as u8;
                let d = (c1 << 4) | c2;
                to.send_data(&[d])?;
            }
        }
        PowerOn.send(to)?;
        DisplayRefresh.send(to)?;
        PowerOff.send(to)?;
        sleep(Duration::from_millis(200));
        Ok(())
    }
}

impl Command for UnknownE3AA {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0xE3)?;
        to.send_data(&[0xAA])
    }
}

impl Command for SetResolution {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x61)?;
        to.send_data(&[0x02, 0x58, 0x01, 0xC0])?;
        Ok(())
    }
}

impl Command for Unknown6022 {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x60)?;
        to.send_data(&[0x22])
    }
}

impl Command for VCOMDataInterval {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x50)?;
        let d = (self.border_output as u8) << 5 | (1 << 4) | 0b0111;
        to.send_data(&[d])?;
        Ok(())
    }
}

impl Command for TempSensor {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x41)?;
        // use internal temp sensor
        to.send_data(&[0x00])?;
        Ok(())
    }
}

impl Command for PLLControl {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x30)?;
        to.send_data(&[0x3C])?;
        Ok(())
    }
}

impl Command for BoosterSoftStart {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x06)?;
        to.send_data(&[0xC7, 0xC7, 0x1D])?;
        Ok(())
    }
}

impl Command for PowerOffSequence {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x03)?;
        to.send_data(&[0x00])?;
        Ok(())
    }
}

impl Command for InternalPower {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x01)?;
        to.send_data(&[0x37, 0x00, 0x23, 0x23])?;
        Ok(())
    }
}

impl Command for PanelSetting {
    fn send(&self, to: &mut impl SpiDevice) -> spi::Result<()> {
        to.send_cmd(0x00)?;
        let d = 0b11100000
            | to_bit(self.ud, 3)
            | to_bit(self.shl, 2)
            | to_bit(self.shd_n, 1)
            | to_bit(self.rst_n, 0);
        to.send_data(&[d, 0x08])?;
        Ok(())
    }
}

impl Default for PanelSetting {
    fn default() -> Self {
        Self {
            ud: true,
            shl: true,
            shd_n: true,
            rst_n: true,
        }
    }
}
