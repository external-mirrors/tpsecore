use lazy_static::lazy_static;
use crate::import::skin_splicer::Piece;
use std::collections::HashMap;

use num_traits::cast::ToPrimitive;

/// A skin slice stores fractional resolution-independent coordinates denoting
/// a slice corresponding to a specific piece. The skin slice then also has a
/// connections map, which returns a sub-slice with the requested connections.
#[derive(Debug, Copy, Clone)]
pub struct SkinSlice {
  pub x: f64, pub y: f64, pub w: f64, pub h: f64,
  pub connections: &'static ConnectionSubmap
}
impl SkinSlice {
  /// Scales and translates the skin slice to exact coordinates on a given image size for a
  /// given connection.
  pub fn slices(self, connection: u8, width: u32, height: u32) -> Option<impl Iterator<Item = (u32, u32, u32, u32)> + 'static> {
    // The part of the image storing the given piece
    // This section will contain one or more connections
    let piece_x = (self.x * width as f64) as u32;
    let piece_y = (self.y * height as f64) as u32;
    let piece_w = (self.w * width as f64) as u32;
    let piece_h = (self.h * height as f64) as u32;

    // The parts of the image storing the given connections
    // Note that multiple connections can be required, as some skins use multiple layers
    // to construct each individual block. (right now just the jstris connected skin).
    Some(self.connections.get(connection)?.iter().cloned().map(move |(conn_pos_x, conn_pos_y)| {
      // the resolution of each individual connection piece
      let conn_w = piece_w as f64 / self.connections.max_x as f64;
      let conn_h = piece_h as f64 / self.connections.max_y as f64;
      // and the location they're located at
      let conn_x = piece_x as f64 + conn_pos_x as f64 * conn_w;
      let conn_y = piece_y as f64 + conn_pos_y as f64 * conn_h;
      log::trace!(
        "Skin slice {} {} {} {} is slicing a {}x{} image for conn {:b}. \
        Piece location: {} {} {} {} -> \
        Slice result: {} {} {} {} \
        (using slice {} {} of max slice map size {} {}).",
        self.x, self.y, self.w, self.h, width, height, connection,
        piece_x, piece_y, piece_w, piece_h,
        conn_x, conn_y, conn_w, conn_h,
        conn_pos_x, conn_pos_y, self.connections.max_x, self.connections.max_y
      );
      (conn_x.to_u32().unwrap(), conn_y.to_u32().unwrap(), conn_w.to_u32().unwrap(), conn_h.to_u32().unwrap())
    }))
  }
}

pub fn tetrio_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Z            => Some(SkinSlice { x:  0.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::L            => Some(SkinSlice { x:  1.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::O            => Some(SkinSlice { x:  2.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::S            => Some(SkinSlice { x:  3.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::I            => Some(SkinSlice { x:  4.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::J            => Some(SkinSlice { x:  5.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::T            => Some(SkinSlice { x:  6.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::Ghost        => Some(SkinSlice { x:  7.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::HoldDisabled => Some(SkinSlice { x:  8.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::Garbage      => Some(SkinSlice { x:  9.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::DarkGarbage  => Some(SkinSlice { x: 10.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
    Piece::Topout       => Some(SkinSlice { x: 11.0/12.0, y: 0.0, w: 1.0/12.4, h: 1.0, connections: &no_conn_submap }),
  }
}
pub fn tetrio_61_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Z            => Some(SkinSlice { x: 0.0 * 6.0/32.0, y: 0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::L            => Some(SkinSlice { x: 1.0 * 6.0/32.0, y: 0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::O            => Some(SkinSlice { x: 2.0 * 6.0/32.0, y: 0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::S            => Some(SkinSlice { x: 3.0 * 6.0/32.0, y: 0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::I            => Some(SkinSlice { x: 4.0 * 6.0/32.0, y: 0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::J            => Some(SkinSlice { x: 0.0 * 6.0/32.0, y: 1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::T            => Some(SkinSlice { x: 1.0 * 6.0/32.0, y: 1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::HoldDisabled => Some(SkinSlice { x: 2.0 * 6.0/32.0, y: 1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::Garbage      => Some(SkinSlice { x: 3.0 * 6.0/32.0, y: 1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::DarkGarbage  => Some(SkinSlice { x: 4.0 * 6.0/32.0, y: 1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &no_conn_submap }),
    Piece::Ghost        => None,
    Piece::Topout       => None
  }
}
pub fn tetrio_61_ghost_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Ghost => Some(SkinSlice { x: 0.0/8.0, y: 0.0, w: 3.0/8.0, h: 3.0/8.0, connections: &no_conn_submap }),
    Piece::Topout => Some(SkinSlice { x: 3.0/8.0, y: 0.0, w: 3.0/8.0, h: 3.0/8.0, connections: &no_conn_submap }),
    _ => None
  }
}
pub fn tetrio_61_conn_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Z            => Some(SkinSlice { x:  0.0 * 6.0/32.0, y:  0.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::L            => Some(SkinSlice { x:  1.0 * 6.0/32.0, y:  0.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::O            => Some(SkinSlice { x:  2.0 * 6.0/32.0, y:  0.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::S            => Some(SkinSlice { x:  3.0 * 6.0/32.0, y:  0.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::I            => Some(SkinSlice { x:  0.0 * 6.0/32.0, y:  1.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::J            => Some(SkinSlice { x:  1.0 * 6.0/32.0, y:  1.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::T            => Some(SkinSlice { x:  2.0 * 6.0/32.0, y:  1.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::HoldDisabled => Some(SkinSlice { x:  3.0 * 6.0/32.0, y:  1.0 * 9.0/32.0, w: 6.0/32.0, h: 9.0/32.0, connections: &tetrio_connections_submap }),
    Piece::Garbage      => Some(SkinSlice { x:  4.0 * 6.0/32.0, y:  0.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &tetrio_garbage_connections_submap }),
    Piece::DarkGarbage  => Some(SkinSlice { x:  4.0 * 6.0/32.0, y:  1.0 * 6.0/32.0, w: 6.0/32.0, h: 6.0/32.0, connections: &tetrio_garbage_connections_submap }),
    Piece::Ghost        => None,
    Piece::Topout       => None
  }
}
pub fn tetrio_61_conn_ghost_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Ghost  => Some(SkinSlice { x: 0.0/16.0, y: 0.0, w: 6.0/16.0, h: 9.0/16.0, connections: &tetrio_connections_submap }),
    Piece::Topout => Some(SkinSlice { x: 6.0/16.0, y: 0.0, w: 6.0/16.0, h: 9.0/16.0, connections: &tetrio_connections_submap }),
    _ => None
  }
}
pub fn jstris_map(piece: Piece) -> Option<SkinSlice> {
  match piece {
    Piece::Z            => Some(SkinSlice { x: 2.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::L            => Some(SkinSlice { x: 3.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::O            => Some(SkinSlice { x: 4.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::S            => Some(SkinSlice { x: 5.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::I            => Some(SkinSlice { x: 6.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::J            => Some(SkinSlice { x: 7.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::T            => Some(SkinSlice { x: 8.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::Ghost        => Some(SkinSlice { x: 1.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::Garbage      => Some(SkinSlice { x: 0.0/9.0, y: 0.0, w: 1.0/9.0, h: 1.0, connections: &no_conn_submap }),
    Piece::HoldDisabled => None,
    Piece::DarkGarbage  => None,
    Piece::Topout       => None
  }
}
pub fn jstris_conn_map(piece: Piece) -> Option<SkinSlice> {
  Some(SkinSlice { connections: &jstris_connections_submap, ..jstris_map(piece)? })
}

/// A map of connections within a piece region
#[derive(Clone, Debug)]
pub struct ConnectionSubmap {
  /// Each connection has a unique index. The overall size of these indexes is based on the
  /// maximum index, which will be used to divide the available space.
  pub connections: HashMap<u8, &'static [(u8, u8)]>,
  /// The default connection that's guaranteed to be present in the map
  pub default: u8,
  /// The maximum x coordinate inserted into `connections`
  pub max_x: u8,
  /// The maximum y coordinate inserted into `connections`
  pub max_y: u8
}

impl ConnectionSubmap {
  pub fn new(default_connection: u8, default_location: &'static [(u8, u8)]) -> Self {
    let mut conn_submap = ConnectionSubmap {
      connections: HashMap::new(),
      default: default_connection,
      max_x: 0,
      max_y: 0
    };
    conn_submap.insert(default_connection, default_location);
    return conn_submap;
  }

  pub fn insert(&mut self, connection: u8, location: &'static [(u8, u8)]) {
    assert!(location.len() > 0, "Expected at least one location");
    self.connections.insert(connection, location);
    self.max_x = self.max_x.max(*location.iter().map(|(x,_)| x).max().unwrap() + 1);
    self.max_y = self.max_y.max(*location.iter().map(|(_,y)| y).max().unwrap() + 1);
  }

  pub fn get(&self, connection: u8) -> Option<&'static [(u8, u8)]> {
    self.connections.get(&connection).map(|val| *val)
  }
}

lazy_static! {
  pub static ref tetrio_connections_submap: ConnectionSubmap = {
    // Include all garbage sides, plus some extra ones.
    let mut map = tetrio_garbage_connections_submap.clone();
    // Corner/elbow sides
    map.insert(0b10110, &[(0, 4)]);
    map.insert(0b10011, &[(1, 4)]);
    map.insert(0b11101, &[(2, 4)]);
    map.insert(0b11110, &[(3, 4)]);
    map.insert(0b11100, &[(0, 5)]);
    map.insert(0b11001, &[(1, 5)]);
    map.insert(0b10111, &[(2, 5)]);
    map.insert(0b11011, &[(3, 5)]);
    return map;
  };
  pub static ref tetrio_garbage_connections_submap: ConnectionSubmap = {
    let mut map = ConnectionSubmap::new(0b00000, &[(0, 3)]);
    // key = corner (T, L, J, S, Z), top, right, bottom, left (1=open,0=closed)
    map.insert(0b00010, &[(0, 0)]);
    map.insert(0b00110, &[(1, 0)]);
    map.insert(0b00111, &[(2, 0)]);
    map.insert(0b00011, &[(3, 0)]);
    map.insert(0b01010, &[(0, 1)]);
    map.insert(0b01110, &[(1, 1)]);
    map.insert(0b01111, &[(2, 1)]);
    map.insert(0b01011, &[(3, 1)]);
    map.insert(0b01000, &[(0, 2)]);
    map.insert(0b01100, &[(1, 2)]);
    map.insert(0b01101, &[(2, 2)]);
    map.insert(0b01001, &[(3, 2)]);
    // map.insert(0b00000, &[(0, 3)]);
    map.insert(0b00100, &[(1, 3)]);
    map.insert(0b00101, &[(2, 3)]);
    map.insert(0b00001, &[(3, 3)]);
    return map;
  };
  pub static ref jstris_connections_submap: ConnectionSubmap = {
    use jstris_dimples::*;
    let mut map = ConnectionSubmap::new(0b00000, &[(0, 0)]);
    map.insert(0b01000, &[(0,  1)]);
    map.insert(0b00010, &[(0,  2)]);
    map.insert(0b01010, &[(0,  3)]);
    map.insert(0b00001, &[(0,  4)]);
    map.insert(0b11001, &[(0,  5)]);
    map.insert(0b10011, &[(0,  6)]);
    map.insert(0b11011, &[(0,  7)]);
    map.insert(0b00100, &[(0,  8)]);
    map.insert(0b11100, &[(0,  9)]);
    map.insert(0b10110, &[(0, 10)]);
    map.insert(0b11110, &[(0, 11)]);
    map.insert(0b00101, &[(0, 12)]);
    map.insert(0b11101, &[(0, 13)]);
    map.insert(0b10111, &[(0, 14)]);
    map.insert(0b11111, &[(0, 15)]);
    map.insert(0b01100, &[(0,  9), TOP_RIGHT]);
    map.insert(0b00110, &[(0, 10), BOTTOM_RIGHT]);
    map.insert(0b00011, &[(0,  6), BOTTOM_LEFT]);
    map.insert(0b01001, &[(0,  5), TOP_LEFT]);
    map.insert(0b00111, &[(0, 14), BOTTOM_RIGHT, BOTTOM_LEFT]);
    map.insert(0b01011, &[(0,  7), TOP_LEFT, BOTTOM_LEFT]);
    map.insert(0b01101, &[(0, 13), TOP_LEFT, TOP_RIGHT]);
    map.insert(0b01110, &[(0, 11), BOTTOM_RIGHT, TOP_RIGHT]);
    map.insert(0b01111, &[(0, 15), TOP_RIGHT, BOTTOM_RIGHT, BOTTOM_LEFT, TOP_LEFT]);
    return map;
  };
  pub static ref no_conn_submap: ConnectionSubmap = {
    ConnectionSubmap::new(0b00000, &[(0, 0)])
  };
}

pub mod jstris_dimples {
  pub const TOP_RIGHT:    (u8, u8) = (0, 16);
  pub const BOTTOM_RIGHT: (u8, u8) = (0, 17);
  pub const BOTTOM_LEFT:  (u8, u8) = (0, 18);
  pub const TOP_LEFT:     (u8, u8) = (0, 19);
}