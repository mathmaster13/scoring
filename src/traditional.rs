#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
pub enum TraditionalJunction {
    V1 = 0b000_000_00, V2 = 0b000_001_01, V3 = 0b000_010_00, V4 = 0b000_011_01, V5 = 0b000_100_00,
    W1 = 0b001_000_01, W2 = 0b001_001_10, W3 = 0b001_010_11, W4 = 0b001_011_10, W5 = 0b001_100_01,
    X1 = 0b010_000_00, X2 = 0b010_001_11, X3 = 0b010_010_00, X4 = 0b010_011_11, X5 = 0b010_100_00,
    Y1 = 0b011_000_01, Y2 = 0b011_001_10, Y3 = 0b011_010_11, Y4 = 0b011_011_10, Y5 = 0b011_100_01,
    Z1 = 0b100_000_00, Z2 = 0b100_001_01, Z3 = 0b100_010_00, Z4 = 0b100_011_01, Z5 = 0b100_100_00
}