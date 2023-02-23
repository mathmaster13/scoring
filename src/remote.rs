use std::collections::HashSet;
use crate::{Junction, Match};
use crate::sealed::Sealed;
use crate::InternalAllianceInfo;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum RemoteCircuitPattern {
    Pattern1 = 0, Pattern2, Pattern3, Pattern4, Pattern5, Pattern6
}

const CIRCUIT_PATTERNS: [HashSet<RedRemoteJunction>; 6] = [
    todo!()
];

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
pub enum RedRemoteJunction {
    X1 = 0b000_000_00, X2 = 0b000_001_01, X3 = 0b000_010_00, X4 = 0b000_011_01, X5 = 0b000_100_00,
    Y1 = 0b001_000_01, Y2 = 0b001_001_10, Y3 = 0b001_010_11, Y4 = 0b001_011_10, Y5 = 0b001_100_01,
    Z1 = 0b010_000_00, Z2 = 0b010_001_01, Z3 = 0b010_010_00, Z4 = 0b010_011_01, Z5 = 0b010_100_00
}

crate::junction_impl!(RedRemoteJunction, 3, 5);

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
// THIS IS INTENTIONALLY BACKWARDS - it allows for red and blue to use the same logic, just with transmutes
pub enum BlueRemoteJunction {
    V1 = 0b010_000_00, V2 = 0b010_001_01, V3 = 0b010_010_00, V4 = 0b010_011_01, V5 = 0b010_100_00,
    W1 = 0b001_000_01, W2 = 0b001_001_10, W3 = 0b001_010_11, W4 = 0b001_011_10, W5 = 0b001_100_01,
    X1 = 0b000_000_00, X2 = 0b000_001_01, X3 = 0b000_010_00, X4 = 0b000_011_01, X5 = 0b000_100_00
}

crate::junction_impl!(BlueRemoteJunction, 3, 5);

// TODO see if optimizing this is worth it
struct InternalRedRemoteMatch {
    data: InternalAllianceInfo<RedRemoteJunction, 1>,
    circuit_pattern: RemoteCircuitPattern,
    has_signal_sleeve: bool
}