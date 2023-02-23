use std::mem::transmute;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum ParkingLocation {
    LeftSignalZone = 0b0100_0000, MiddleSignalZone, RightSignalZone,
    NearTerminal = 0b1000_0000, FarTerminal,
    Substation = 0b0010_0000
}

impl ParkingLocation {
    #[inline(always)]
    pub(crate) fn is_signal_zone(self) -> bool {
        self as u8 & 0b0100_0000 != 0
    }

    #[inline(always)]
    pub(crate) fn is_terminal(self) -> bool {
        self as u8 & 0b1000_0000 != 0
    }
}

impl From<Terminal> for ParkingLocation {
    fn from(value: Terminal) -> Self {
        unsafe { transmute(value) }
    }
}

impl From<SignalZone> for ParkingLocation {
    fn from(value: SignalZone) -> Self {
        unsafe { transmute(value) }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum Terminal {
    Near = 0b1000_0000, Far
}

impl TryFrom<ParkingLocation> for Terminal {
    type Error = ();

    fn try_from(value: ParkingLocation) -> Result<Self, Self::Error> {
        if value.is_terminal() {
            Ok(unsafe { transmute(value) })
        } else {
            Err(())
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum SignalZone {
    Left = 0b0100_0000, Middle, Right
}

impl TryFrom<ParkingLocation> for SignalZone {
    type Error = ();

    fn try_from(value: ParkingLocation) -> Result<Self, Self::Error> {
        if value.is_signal_zone() {
            Ok(unsafe { transmute(value) })
        } else {
            Err(())
        }
    }
}