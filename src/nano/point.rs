use std::cmp;

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Pn {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl Pn {
    pub fn zero() -> Pn {
        Pn { x: 0, y: 0, z: 0 }
    }
    pub fn add(self, rhs: PnDiff, r: u8) -> Option<Pn> {
        let r = r as i16;
        let x = (self.x as i16) + (rhs.dx as i16);
        let y = (self.y as i16) + (rhs.dy as i16);
        let z = (self.z as i16) + (rhs.dz as i16);
        if x >= 0 && x < r && y >= 0 && y < r && z >= 0 && z < r {
            Some(Pn {
                x: x as u8,
                y: y as u8,
                z: z as u8,
            })
        } else {
            None
        }
    }

    pub fn diff(self, rhs: Pn) -> (i16, i16, i16) {
        let dx = (self.x as i16) - (rhs.x as i16);
        let dy = (self.y as i16) - (rhs.y as i16);
        let dz = (self.z as i16) - (rhs.z as i16);
        (dx, dy, dz)
    }

    pub fn region(self, np: Pn) -> Vec<Pn> {
        let lx = cmp::min(self.x, np.x);
        let rx = cmp::max(self.x, np.x);
        let ly = cmp::min(self.y, np.y);
        let ry = cmp::max(self.y, np.y);
        let lz = cmp::min(self.z, np.z);
        let rz = cmp::max(self.z, np.z);
        let mut res = Vec::new();
        for x in lx..rx + 1 {
            for y in ly..ry + 1 {
                for z in lz..rz + 1 {
                    res.push(Pn { x, y, z });
                }
            }
        }
        res
    }

    pub fn adjacents(self, r: u8) -> impl Iterator<Item = Pn> {
        PnDiff::adjacents().filter_map(move |dp| self.add(dp, r))
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct PnDiff {
    pub dx: i8,
    pub dy: i8,
    pub dz: i8,
}

impl PnDiff {
    pub fn new(dx: i8, dy: i8, dz: i8) -> PnDiff {
        PnDiff { dx, dy, dz }
    }
    pub fn from_i16((dx, dy, dz): (i16, i16, i16)) -> PnDiff {
        PnDiff {
            dx: dx as i8,
            dy: dy as i8,
            dz: dz as i8,
        }
    }
    fn adjacents() -> impl Iterator<Item = PnDiff> {
        lazy_static! {
            static ref ADJS: Vec<PnDiff> = vec![
                PnDiff {
                    dx: 1,
                    dy: 0,
                    dz: 0,
                },
                PnDiff {
                    dx: -1,
                    dy: 0,
                    dz: 0,
                },
                PnDiff {
                    dx: 0,
                    dy: 1,
                    dz: 0,
                },
                PnDiff {
                    dx: 0,
                    dy: -1,
                    dz: 0,
                },
                PnDiff {
                    dx: 0,
                    dy: 0,
                    dz: 1,
                },
                PnDiff {
                    dx: 0,
                    dy: 0,
                    dz: -1,
                },
            ];
        }
        ADJS.iter().cloned()
    }

    pub fn mlen(&self) -> u8 {
        self.dx.abs() as u8 + self.dy.abs() as u8 + self.dz.abs() as u8
    }

    pub fn clen(&self) -> u8 {
        cmp::max(
            cmp::max(self.dx.abs() as u8, self.dy.abs() as u8),
            self.dz.abs() as u8,
        )
    }

    pub fn decode_short_linear(a: u8, i: u8) -> PnDiff {
        Self::decode_linear(a, i, 5)
    }
    pub fn decode_long_linear(a: u8, i: u8) -> PnDiff {
        Self::decode_linear(a, i, 15)
    }
    pub fn decode_near(nd: u8) -> PnDiff {
        let dz = ((nd / 1) % 3) as i8 - 1;
        let dy = ((nd / 3) % 3) as i8 - 1;
        let dx = (nd / 9) as i8 - 1;
        PnDiff { dx, dy, dz }
    }
    pub fn decode_far(a: u8, b: u8, c: u8) -> PnDiff {
        let dx = (a as i8) - 30;
        let dy = (b as i8) - 30;
        let dz = (c as i8) - 30;
        PnDiff { dx, dy, dz }
    }

    pub fn encode_short_linear(&self) -> (u8, u8) {
        self.encode_linear(5)
    }
    pub fn encode_long_linear(&self) -> (u8, u8) {
        self.encode_linear(15)
    }
    // TODO: Return Option rather than assert ?
    pub fn encode_near(&self) -> u8 {
        assert!(self.is_near());
        ((self.dx + 1) * 9 + (self.dy + 1) * 3 + (self.dz + 1)) as u8
    }

    pub fn encode_far(&self) -> (u8, u8, u8) {
        assert!(self.is_far());
        (
            (self.dx + 30) as u8,
            (self.dy + 30) as u8,
            (self.dz + 30) as u8,
        )
    }

    fn decode_linear(a: u8, i: u8, size: i8) -> PnDiff {
        assert!(i <= 2 * size as u8);
        match a {
            0b01 => PnDiff {
                dx: (i as i8 - size),
                dy: 0,
                dz: 0,
            },
            0b10 => PnDiff {
                dx: 0,
                dy: (i as i8 - size),
                dz: 0,
            },
            0b11 => PnDiff {
                dx: 0,
                dy: 0,
                dz: (i as i8 - size),
            },
            _ => unreachable!(),
        }
    }

    fn encode_linear(&self, size: u8) -> (u8, u8) {
        assert!(self.is_linear(size));

        if self.dx != 0 {
            (0b01, (self.dx + size as i8) as u8)
        } else if self.dy != 0 {
            (0b10, (self.dy + size as i8) as u8)
        } else if self.dz != 0 {
            (0b11, (self.dz + size as i8) as u8)
        } else {
            unreachable!()
        }
    }

    fn is_linear(&self, size: u8) -> bool {
        if self.dx != 0 {
            self.dy == 0 && self.dz == 0 && self.dx.abs() > 0 && self.dx.abs() <= size as i8
        } else if self.dy != 0 {
            self.dx == 0 && self.dz == 0 && self.dy.abs() > 0 && self.dy.abs() <= size as i8
        } else if self.dz != 0 {
            self.dx == 0 && self.dy == 0 && self.dz.abs() > 0 && self.dz.abs() <= size as i8
        } else {
            false
        }
    }

    pub fn is_long_linear(&self) -> bool {
        self.is_linear(15)
    }

    pub fn is_short_linear(&self) -> bool {
        self.is_linear(15)
    }

    pub fn is_near(&self) -> bool {
        self.mlen() > 0 && self.mlen() <= 2 && self.clen() == 1
    }

    pub fn is_far(&self) -> bool {
        self.clen() > 0 && self.clen() <= 30
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_long_linear() {
        let tests = vec![
            PnDiff {
                dx: 15,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: -15,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: 1,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: -1,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dy: 15,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: -15,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: 1,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: -1,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dz: 15,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: -15,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: 1,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: -1,
                dx: 0,
                dy: 0,
            },
        ];
        for t in tests {
            let (a, i) = t.encode_long_linear();
            assert_eq!(t, PnDiff::decode_long_linear(a, i));
        }
    }

    #[test]
    fn test_encode_short_linear() {
        let tests = vec![
            PnDiff {
                dx: 5,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: -5,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: 1,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dx: -1,
                dy: 0,
                dz: 0,
            },
            PnDiff {
                dy: 5,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: -5,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: 1,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dy: -1,
                dx: 0,
                dz: 0,
            },
            PnDiff {
                dz: 5,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: -5,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: 1,
                dx: 0,
                dy: 0,
            },
            PnDiff {
                dz: -1,
                dx: 0,
                dy: 0,
            },
        ];
        for t in tests {
            let (a, i) = t.encode_short_linear();
            assert_eq!(t, PnDiff::decode_short_linear(a, i));
        }
    }

    #[test]
    fn test_encode_near() {
        let tests = vec![
            PnDiff {
                dx: -1,
                dy: 1,
                dz: 0,
            },
            PnDiff {
                dx: 0,
                dy: 1,
                dz: 1,
            },
            PnDiff {
                dx: 0,
                dy: -1,
                dz: -1,
            },
            PnDiff {
                dx: 1,
                dy: 0,
                dz: -1,
            },
            PnDiff {
                dx: 0,
                dy: 1,
                dz: 0,
            },
            PnDiff {
                dx: 0,
                dy: -1,
                dz: 0,
            },
            PnDiff {
                dx: 0,
                dy: 0,
                dz: -1,
            },
            PnDiff {
                dx: 1,
                dy: 0,
                dz: 0,
            },
        ];
        for t in tests {
            let nd = t.encode_near();
            assert_eq!(t, PnDiff::decode_near(nd));
        }
    }
}
