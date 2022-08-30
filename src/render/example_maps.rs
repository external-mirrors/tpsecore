use crate::import::skin_splicer::Piece;

const E: Option<(Piece, u8)> = None;
const fn z(conn: u8) -> Option<(Piece, u8)> { Some((Piece::Z, conn)) }
const fn l(conn: u8) -> Option<(Piece, u8)> { Some((Piece::L, conn)) }
const fn o(conn: u8) -> Option<(Piece, u8)> { Some((Piece::O, conn)) }
const fn s(conn: u8) -> Option<(Piece, u8)> { Some((Piece::S, conn)) }
const fn i(conn: u8) -> Option<(Piece, u8)> { Some((Piece::I, conn)) }
const fn j(conn: u8) -> Option<(Piece, u8)> { Some((Piece::J, conn)) }
const fn t(conn: u8) -> Option<(Piece, u8)> { Some((Piece::T, conn)) }
const fn p(conn: u8) -> Option<(Piece, u8)> { Some((Piece::Ghost, conn)) }
const fn g(conn: u8) -> Option<(Piece, u8)> { Some((Piece::Garbage, conn)) }
const fn d(conn: u8) -> Option<(Piece, u8)> { Some((Piece::DarkGarbage, conn)) }
const fn w(conn: u8) -> Option<(Piece, u8)> { Some((Piece::Topout, conn)) }

pub const EMPTY_MAP: &[&[Option<(Piece, u8)>]] = &[
  &[E, E, E, E, E, E, E, E, E, E], // 24 (skyline)
  &[E, E, E, E, E, E, E, E, E, E], // 23 (skyline)
  &[E, E, E, E, E, E, E, E, E, E], // 22 (skyline)
  &[E, E, E, E, E, E, E, E, E, E], // 21 (skyline)
  &[E, E, E, E, E, E, E, E, E, E], // 20
  &[E, E, E, E, E, E, E, E, E, E], // 19
  &[E, E, E, E, E, E, E, E, E, E], // 18
  &[E, E, E, E, E, E, E, E, E, E], // 17
  &[E, E, E, E, E, E, E, E, E, E], // 16
  &[E, E, E, E, E, E, E, E, E, E], // 15
  &[E, E, E, E, E, E, E, E, E, E], // 14
  &[E, E, E, E, E, E, E, E, E, E], // 13
  &[E, E, E, E, E, E, E, E, E, E], // 12
  &[E, E, E, E, E, E, E, E, E, E], // 11
  &[E, E, E, E, E, E, E, E, E, E], // 10
  &[E, E, E, E, E, E, E, E, E, E], // 9
  &[E, E, E, E, E, E, E, E, E, E], // 8
  &[E, E, E, E, E, E, E, E, E, E], // 7
  &[E, E, E, E, E, E, E, E, E, E], // 6
  &[E, E, E, E, E, E, E, E, E, E], // 5
  &[E, E, E, E, E, E, E, E, E, E], // 4
  &[E, E, E, E, E, E, E, E, E, E], // 3
  &[E, E, E, E, E, E, E, E, E, E], // 2
  &[E, E, E, E, E, E, E, E, E, E], // 1
];

// todo: update to new connection logic
pub const PCO_MAP: &[&[Option<(Piece, u8)>]] = &[
  &[         E,          E,          E,          E,          E, w(0b00010),          E,          E,          E,          E], // 24 (skyline)
  &[         E,          E,          E,          E,          E, w(0b01010),          E,          E,          E,          E], // 23 (skyline)
  &[         E,          E,          E,          E,          E, w(0b01010),          E,          E,          E,          E], // 22 (skyline)
  &[         E,          E,          E,          E,          E, w(0b01000),          E,          E,          E,          E], // 21 (skyline)
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 20
  &[         E,          E,          E,          E,          E, t(0b00010),          E,          E,          E,          E], // 19
  &[         E,          E,          E,          E, t(0b00100), t(0b11011),          E,          E,          E,          E], // 18
  &[         E,          E,          E,          E,          E, t(0b01000),          E,          E,          E,          E], // 17
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 16
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 15
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 14
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 13
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 12
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 11
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 10
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 9
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 8
  &[         E,          E,          E,          E,          E,          E,          E,          E,          E,          E], // 7
  &[z(0b00100), z(0b10011),          E,          E,          E,          E, i(0b00100), i(0b00101), i(0b00101), i(0b00001)], // 6
  &[t(0b00010), z(0b11100), z(0b00001),          E,          E, p(0b00010), l(0b00010), o(0b00110), o(0b00011), j(0b00010)], // 5
  &[t(0b11110), t(0b00001), s(0b10110), s(0b00001), p(0b00100), p(0b11011), l(0b01010), o(0b01100), o(0b01001), j(0b01010)], // 4
  &[t(0b01000), s(0b00100), s(0b11001),          E,          E, p(0b01000), l(0b11100), l(0b00001), j(0b00100), j(0b11001)], // 3
  &[g(0b00100), g(0b00101), g(0b00101), g(0b00001),          E, g(0b00100), g(0b00101), g(0b00101), g(0b00101), g(0b00001)], // 2
  &[d(0b00100), d(0b00101), d(0b00101), d(0b00101), d(0b00101), d(0b00101), d(0b00101), d(0b00101), d(0b00101), d(0b00001)], // 1
];
