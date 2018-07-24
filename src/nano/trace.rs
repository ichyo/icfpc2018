use nano::util::{is_suffix, mask, read_u8};
use nano::*;
use std::io;
use std::io::prelude::*;
use std::vec;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Command {
    Halt,
    Wait,
    Flip,
    SMove(PnDiff),
    LMove(PnDiff, PnDiff),
    Fission(PnDiff, u8),
    Fill(PnDiff),
    FusionP(PnDiff),
    FusionS(PnDiff),
    Void(PnDiff),

    GFill(PnDiff, PnDiff),
    GVoid(PnDiff, PnDiff),
}

fn invalid_coord_msg(c: Pn, d: PnDiff) -> String {
    format!("invalid coordinate: {:?} {:?}", c, d)
}

impl Command {
    pub fn volatile_points(&self, c: Pn, r: u8) -> Result<Vec<Pn>> {
        Ok(match self {
            Command::Halt => vec![c],
            Command::Wait => vec![c],
            Command::Flip => vec![c],
            Command::SMove(d) => {
                let t = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                c.region(t)
            }
            Command::LMove(d1, d2) => {
                let m = c.add(*d1, r).ok_or(invalid_coord_msg(c, *d1))?;
                let t = m.add(*d2, r).ok_or(invalid_coord_msg(m, *d2))?;
                let xs = c.region(m);
                let ys = m.region(t);
                [&xs[..], &ys[..]].concat()
            }
            Command::Fission(d, _) => {
                let t = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                vec![c, t]
            }
            Command::Fill(d) => {
                let t = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                vec![c, t]
            }
            Command::Void(d) => {
                let t = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                vec![c, t]
            }
            Command::FusionP(d) => {
                let _ = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                vec![c]
            }
            Command::FusionS(d) => {
                let _ = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                vec![c]
            }
            Command::GFill(d, f) => {
                let r1 = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                let r2 = r1.add(*f, r).ok_or(invalid_coord_msg(c, *d))?;
                let mut res = vec![c];
                res.extend(r1.region(r2));
                res
            }
            Command::GVoid(d, f) => {
                let r1 = c.add(*d, r).ok_or(invalid_coord_msg(c, *d))?;
                let r2 = r1.add(*f, r).ok_or(invalid_coord_msg(c, *d))?;
                let mut res = vec![c];
                res.extend(r1.region(r2));
                res
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Trace(Vec<Command>);

impl Trace {
    pub fn new(cmds: Vec<Command>) -> Trace {
        Trace(cmds)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Command> {
        self.0.iter()
    }
}

impl IntoIterator for Trace {
    type Item = Command;
    type IntoIter = vec::IntoIter<Command>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Trace {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        for cmd in &self.0 {
            match cmd {
                Command::Halt => w.write(&[0b11111111])?,
                Command::Wait => w.write(&[0b11111110])?,
                Command::Flip => w.write(&[0b11111101])?,
                Command::SMove(p) => {
                    let (a, i) = p.encode_long_linear();
                    w.write(&[0b00000100 | (a << 4), i])?
                }
                Command::LMove(p1, p2) => {
                    let (a1, i1) = p1.encode_short_linear();
                    let (a2, i2) = p2.encode_short_linear();
                    w.write(&[(a2 << 6) | (a1 << 4) | 0b1100, (i2 << 4) | i1])?
                }
                Command::FusionP(p) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b111])?
                }
                Command::FusionS(p) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b110])?
                }
                Command::Fission(p, m) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b101, *m])?
                }
                Command::Fill(p) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b011])?
                }
                Command::Void(p) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b010])?
                }
                Command::GFill(p, f) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b001])?;
                    let (fx, fy, fz) = f.encode_far();
                    w.write(&[fx, fy, fz])?
                }
                Command::GVoid(p, f) => {
                    let nd = p.encode_near();
                    w.write(&[(nd << 3) | 0b000])?;
                    let (fx, fy, fz) = f.encode_far();
                    w.write(&[fx, fy, fz])?
                }
            };
        }
        Ok(())
    }
    pub fn read<R: Read>(r: &mut R) -> io::Result<Trace> {
        let mut res = Vec::new();
        loop {
            let c = read_u8(r)?;
            if c.is_none() {
                break;
            }
            let x = c.unwrap();
            let cmd = match x {
                0b11111111 => Command::Halt,
                0b11111110 => Command::Wait,
                0b11111101 => Command::Flip,
                x if is_suffix(x, 0b0100, 4) => {
                    let y = read_u8(r)?.unwrap();
                    let a = mask(x >> 4, 2);
                    let i = mask(y, 5);
                    let dp = PnDiff::decode_long_linear(a, i);
                    Command::SMove(dp)
                }
                x if is_suffix(x, 0b1100, 4) => {
                    let y = read_u8(r)?.unwrap();
                    let a1 = mask(x >> 4, 2);
                    let a2 = mask(x >> 6, 2);
                    let i1 = mask(y, 4);
                    let i2 = mask(y >> 4, 4);
                    let dp1 = PnDiff::decode_short_linear(a1, i1);
                    let dp2 = PnDiff::decode_short_linear(a2, i2);
                    Command::LMove(dp1, dp2)
                }
                x if is_suffix(x, 0b111, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    Command::FusionP(p)
                }
                x if is_suffix(x, 0b110, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    Command::FusionS(p)
                }
                x if is_suffix(x, 0b101, 3) => {
                    let y = read_u8(r)?.unwrap();
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    let m = y;
                    Command::Fission(p, m)
                }
                x if is_suffix(x, 0b011, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    Command::Fill(p)
                }
                x if is_suffix(x, 0b010, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    Command::Void(p)
                }
                x if is_suffix(x, 0b001, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    let dx = read_u8(r)?.unwrap();
                    let dy = read_u8(r)?.unwrap();
                    let dz = read_u8(r)?.unwrap();
                    let f = PnDiff::decode_far(dx, dy, dz);
                    Command::GFill(p, f)
                }
                x if is_suffix(x, 0b000, 3) => {
                    let nd = mask(x >> 3, 5);
                    let p = PnDiff::decode_near(nd);
                    let dx = read_u8(r)?.unwrap();
                    let dy = read_u8(r)?.unwrap();
                    let dz = read_u8(r)?.unwrap();
                    let f = PnDiff::decode_far(dx, dy, dz);
                    Command::GVoid(p, f)
                }
                _ => unreachable!(),
            };
            res.push(cmd);
        }
        Ok(Trace(res))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_write() {
        let trace = Trace(vec![
            Command::Halt,
            Command::Wait,
            Command::Flip,
            Command::SMove(PnDiff {
                dx: 0,
                dy: 0,
                dz: 15,
            }),
            Command::SMove(PnDiff {
                dx: -15,
                dy: 0,
                dz: 0,
            }),
            Command::LMove(
                PnDiff {
                    dx: 0,
                    dy: 0,
                    dz: 5,
                },
                PnDiff {
                    dx: -5,
                    dy: 0,
                    dz: 0,
                },
            ),
            Command::FusionP(PnDiff {
                dx: 0,
                dy: 1,
                dz: -1,
            }),
            Command::FusionS(PnDiff {
                dx: 1,
                dy: 0,
                dz: 0,
            }),
            Command::Fission(
                PnDiff {
                    dx: 1,
                    dy: 1,
                    dz: 0,
                },
                10,
            ),
            Command::Fill(PnDiff {
                dx: 0,
                dy: -1,
                dz: 0,
            }),
            Command::GFill(
                PnDiff {
                    dx: 0,
                    dy: -1,
                    dz: 0,
                },
                PnDiff {
                    dx: 30,
                    dy: 30,
                    dz: 30,
                },
            ),
            Command::GVoid(
                PnDiff {
                    dx: 1,
                    dy: 0,
                    dz: 0,
                },
                PnDiff {
                    dx: -30,
                    dy: -30,
                    dz: -30,
                },
            ),
        ]);
        let mut buf = Vec::new();
        trace.write(&mut buf).unwrap();
        assert_eq!(trace, Trace::read(&mut Cursor::new(buf)).unwrap());
    }
}
