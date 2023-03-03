use std::mem::transmute;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum ParkingLocation {
    // signal zones
    LeftSignalZone = 0b0100_0000, MiddleSignalZone, RightSignalZone,
    // terminals
    NearTerminal = 0b1000_0000, FarTerminal,
    // other
    Substation = 0b0010_0000
}
crate::display_impl_as_debug!(ParkingLocation);

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
    #[inline(always)]
    fn from(value: Terminal) -> Self {
        unsafe { transmute(value) }
    }
}

impl From<SignalZone> for ParkingLocation {
    #[inline(always)]
    fn from(value: SignalZone) -> Self {
        unsafe { transmute(value) }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum Terminal {
    Near = 0b1000_0000, Far
}
crate::display_impl_as_debug!(Terminal);

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
    Left = 0b0100_0000,
    Middle,
    Right,
}
crate::display_impl_as_debug!(SignalZone);

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
