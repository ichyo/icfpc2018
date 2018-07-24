use super::util::{floor, read_u8, reverse_bits};
use bit_set::BitSet;
use nano::*;
use std::collections::{HashSet, VecDeque};
use std::io;
use std::io::prelude::*;

#[derive(Clone, Debug)]
pub struct Model(Matrix);

#[derive(Clone, Debug)]
pub struct Matrix {
    r: u8,
    bits: BitSet,
}

impl Matrix {
    pub fn new(r: u8) -> Matrix {
        let rb = r as usize;
        let n = floor(rb * rb * rb, 8);
        Matrix {
            r,
            bits: BitSet::with_capacity(n),
        }
    }
    pub fn from_model(m: &Model) -> Matrix {
        Matrix {
            r: m.0.r,
            bits: m.0.bits.clone(),
        }
    }
    pub fn read<R: Read>(reader: &mut R) -> io::Result<Matrix> {
        let r = match read_u8(reader)? {
            Some(r) => r as usize,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "cannot read R",
                ))
            }
        };
        let n = floor(r * r * r, 8);
        let mut buffer = vec![0; n];
        reader.read_exact(&mut buffer)?;
        for i in 0..n {
            buffer[i] = reverse_bits(buffer[i]);
        }
        let bits = BitSet::from_bytes(&mut buffer);
        Ok(Matrix { r: r as u8, bits })
    }

    pub fn get(&self, p: Pn) -> bool {
        if p.x >= self.r || p.y >= self.r || p.z >= self.r {
            return false;
        }
        let r = self.r as usize;
        let index = (p.x as usize) * r * r + (p.y as usize) * r + (p.z as usize);
        self.bits.contains(index)
    }

    pub fn set(&mut self, p: Pn) -> bool {
        if p.x >= self.r || p.y >= self.r || p.z >= self.r {
            panic!("invalid pos");
        }
        let r = self.r as usize;
        let index = (p.x as usize) * r * r + (p.y as usize) * r + (p.z as usize);
        self.bits.insert(index)
    }

    pub fn unset(&mut self, p: Pn) -> bool {
        if p.x >= self.r || p.y >= self.r || p.z >= self.r {
            panic!("invalid pos");
        }
        let r = self.r as usize;
        let index = (p.x as usize) * r * r + (p.y as usize) * r + (p.z as usize);
        self.bits.remove(index)
    }

    fn index_to_point(&self, index: usize) -> Pn {
        let r = self.r as usize;
        Pn {
            x: (index / r / r) as u8,
            y: (index / r % r) as u8,
            z: (index % r) as u8,
        }
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }

    pub fn full_points(&self) -> Vec<Pn> {
        self.bits.iter().map(|x| self.index_to_point(x)).collect()
    }

    pub fn diff_points(&self, other: &Matrix) -> Vec<Pn> {
        self.bits
            .symmetric_difference(&other.bits)
            .map(|x| self.index_to_point(x))
            .collect()
    }

    pub fn is_grounded(&self) -> bool {
        let mut queue = VecDeque::new();
        let mut set = HashSet::new();
        let points = self.full_points();
        for p in points.iter().filter(|p| p.y == 0) {
            queue.push_back(*p);
            set.insert(*p);
        }
        while let Some(p) = queue.pop_front() {
            for np in p.adjacents(self.r) {
                if self.get(np) && !set.contains(&np) {
                    queue.push_back(np);
                    set.insert(np);
                }
            }
        }
        for p in points {
            if !set.contains(&p) {
                return false;
            }
        }
        true
    }

    pub fn resolution(&self) -> u8 {
        self.r
    }
}

impl Model {
    pub fn new(r: u8) -> Model {
        Model(Matrix::new(r))
    }

    pub fn read<R: Read>(reader: &mut R) -> io::Result<Model> {
        Ok(Model(Matrix::read(reader)?))
    }

    pub fn get(&self, p: Pn) -> bool {
        self.0.get(p)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn full_points(&self) -> Vec<Pn> {
        self.0.full_points()
    }

    pub fn diff_points(&self, other: &Model) -> Vec<Pn> {
        self.0.diff_points(&other.0)
    }

    pub fn diff_points_m(&self, other: &Matrix) -> Vec<Pn> {
        self.0.diff_points(other)
    }

    pub fn resolution(&self) -> u8 {
        self.0.resolution()
    }

    pub fn is_well_formed(&self) -> bool {
        let r = self.0.r;
        for p in self.0.full_points() {
            if !(0 < p.x && p.x < r - 1 && p.y < r - 1 && 1 <= p.z && p.z < r - 1) {
                return false;
            }
        }
        self.0.is_grounded()
    }

    pub fn is_complete(&self, matrix: &Matrix) -> bool {
        self.0.bits == matrix.bits
    }
}
