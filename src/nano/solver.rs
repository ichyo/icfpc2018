use nano::util::floor;
use nano::*;
use rand::{thread_rng, Rng, RngCore};
use std::cmp;
use std::collections::{HashSet, VecDeque};
use std::iter::repeat;

pub struct Solver<'a> {
    game: Game<'a>,
    model_src: &'a Model,
    model_dst: &'a Model,
    commands: Vec<Command>,
    cmd_queue_by_bots: Vec<VecDeque<Command>>,
    prob: usize,
    iter: usize,
    num_bots: u8,
    max_group: usize,
}

const MAX_SEED: u8 = 40;
const MAX_ITER: usize = 2000000;

impl<'a> Solver<'a> {
    fn new(
        model_src: &'a Model,
        model_dst: &'a Model,
        prob: usize,
        num_bots: u8,
        max_group: usize,
    ) -> Solver<'a> {
        let game = Game::new(model_src, model_dst);
        Solver {
            game,
            model_src,
            model_dst,
            commands: Vec::new(),
            cmd_queue_by_bots: repeat(VecDeque::new()).take(num_bots as usize).collect(),
            prob,
            iter: 0,
            num_bots,
            max_group,
        }
    }

    fn exec_single(&mut self, command: Command) -> Result<()> {
        self.commands.push(command);
        self.game.execute(&Trace::new(vec![command]))
    }

    fn exec_all(&mut self, commands: Vec<Command>) -> Result<()> {
        if commands.iter().all(|c| match c {
            Command::Wait => true,
            _ => false,
        }) {
            //return Err("all wait".into());
        }

        self.commands.extend(commands.clone());
        self.game.execute(&Trace::new(commands))?;
        self.iter += 1;
        if self.iter >= MAX_ITER {
            return Err("max iter".into());
        }
        Ok(())
    }

    fn start_point(&self, idx: u8) -> Pn {
        let r = self.game.resolution();
        if MAX_SEED as usize <= 2 * r as usize - 1 {
            if idx < r {
                Pn {
                    x: idx as u8,
                    y: idx as u8,
                    z: 0,
                }
            } else {
                let nidx = idx - (r - 1);
                Pn {
                    x: r - 1 - nidx,
                    y: r - 1,
                    z: nidx,
                }
            }
        } else if MAX_SEED as usize <= 3 * r as usize - 3 {
            if idx < r {
                Pn {
                    x: idx as u8,
                    y: idx as u8,
                    z: 0,
                }
            } else if idx < r + r - 1 {
                let nidx = idx - (r - 1);
                Pn {
                    x: r - 1,
                    y: r - 1,
                    z: nidx,
                }
            } else {
                let nidx = idx - (r - 1 + r - 1);
                Pn {
                    x: r - 1 - nidx,
                    y: r - 1,
                    z: r - 1,
                }
            }
        } else {
            unreachable!()
        }
    }

    fn setup_all(&mut self) -> Result<()> {
        self.exec_single(Command::Flip)?;

        for i in 1..self.num_bots {
            let mut commands = Vec::new();
            commands.extend(repeat(Command::Wait).take(i as usize - 1));
            let pd = self.start_point(i).diff(self.start_point(i - 1));
            let pd = PnDiff {
                dx: pd.0 as i8,
                dy: pd.1 as i8,
                dz: pd.2 as i8,
            };
            commands.push(Command::Fission(pd, MAX_SEED - i as u8 - 1));
            self.exec_all(commands)?;
        }

        Ok(())
    }

    fn finish_all(&mut self) -> Result<()> {
        let num_bots = self.game.bots().len() as u8;

        let targets = (0..num_bots)
            .map(|i| Some(self.start_point(i as u8)))
            .collect::<Vec<_>>();
        loop {
            if self.arrived_bots(&targets).len() == num_bots as usize {
                break;
            }
            let cmds: Vec<_> = self.calc_next_commands(&targets)?.0;

            self.exec_all(cmds)?;
        }

        for i in (2..=num_bots).rev() {
            let mut commands = Vec::new();
            commands.extend(repeat(Command::Wait).take(i as usize - 2));
            let pd = self.start_point(i - 1).diff(self.start_point(i - 2));
            let pd = (pd.0 as i8, pd.1 as i8, pd.2 as i8);
            commands.push(Command::FusionP(PnDiff::new(pd.0, pd.1, pd.2)));
            commands.push(Command::FusionS(PnDiff::new(-pd.0, -pd.1, -pd.2)));
            self.exec_all(commands)?;
        }

        self.exec_single(Command::Flip)?;
        self.exec_single(Command::Halt)?;
        Ok(())
    }

    fn arrived_bots(&self, target: &Vec<Option<Pn>>) -> Vec<usize> {
        let bots = self.game.bots();
        let n = bots.len();
        (0..n)
            .filter(|&i| target[i].is_some())
            .filter(|&i| bots[i].pos == target[i].unwrap() && self.cmd_queue_by_bots[i].is_empty())
            .collect()
    }

    fn calc_next_commands(
        &mut self,
        target: &Vec<Option<Pn>>,
    ) -> Result<(Vec<Command>, HashSet<Pn>)> {
        let mut volatile_set: HashSet<Pn> = HashSet::new();
        volatile_set.extend(self.game.bots().iter().map(|b| b.pos));
        let bots = self.game.bots();

        for i in 0..bots.len() {
            let cur = bots[i].pos;
            if let Some(t) = target[i] {
                let cs = self.move_single(cur, t)?;
                let mut c_que = &mut self.cmd_queue_by_bots[i];
                if c_que.is_empty() {
                    //eprintln!("{:?} update {:?} for {:?}", bot, cs, t);
                    c_que.extend(cs);
                }
            }
        }

        let mut res = Vec::new();
        for i in 0..bots.len() {
            let cur = bots[i].pos;
            let mut pickup = false;
            let mut c_que = &mut self.cmd_queue_by_bots[i];
            let cmd = if let Some(&t) = c_que.front() {
                let ps = t.volatile_points(cur, self.game.resolution())?;
                if ps.iter().any(|p| volatile_set.contains(p) && *p != cur) {
                    Command::Wait
                } else {
                    pickup = true;
                    volatile_set.extend(ps);
                    t
                }
            } else {
                Command::Wait
            };

            if pickup {
                c_que.pop_front();
            } else {
                c_que.clear();
            }

            res.push(cmd);
        }

        Ok((res, volatile_set))
    }

    fn calc_points_by_bots(&self) -> Vec<VecDeque<Pn>> {
        let mut all_points = self.game.diff_points();
        all_points.sort_by_key(|p| (p.y, p.x, p.z));

        let num_bots = self.game.bots().len();

        let mut points_by_bots = Vec::new();
        let mut begin = 0;
        for i in 0..num_bots {
            let mut size = all_points.len() / num_bots + if i < all_points.len() % num_bots {
                1
            } else {
                0
            };
            let mut points = Vec::new();
            for p in &all_points[begin..begin + size] {
                points.push(p.clone());
            }
            points_by_bots.push(points);
            begin += size;
        }

        points_by_bots
            .into_iter()
            .map(|ps| ps.into_iter().collect::<VecDeque<_>>())
            .collect::<Vec<_>>()
    }

    fn exec_all_remove(&mut self) -> Result<()> {
        let boxes = enumerate_remove_box(&self.model_src)?;

        let mut next_boxes = boxes.iter().collect::<VecDeque<_>>();
        let mut waiting: HashSet<([usize; 8], (Pn, Pn))> = HashSet::new();
        let mut targets: Vec<Option<Pn>> = repeat(None)
            .take(self.num_bots as usize)
            .collect::<Vec<_>>();
        loop {
            if next_boxes.is_empty() && waiting.is_empty() {
                break;
            }

            let n = self.num_bots as usize;
            let mut free_bots: VecDeque<usize> = (0..n).filter(|i| targets[*i].is_none()).collect();
            while let Some(_) = next_boxes.front() {
                if free_bots.len() < 8 || waiting.len() >= self.max_group {
                    break;
                }

                let (fp, tp) = next_boxes.pop_front().unwrap();

                let mut use_bots = [0; 8];
                assert!(free_bots.len() >= 8);
                for i in 0..8 {
                    use_bots[i] = free_bots.pop_front().unwrap();
                }

                for s in 0..8 {
                    let x = if ((s >> 0) & 1) == 1 {
                        tp.x
                    } else {
                        fp.x
                    };
                    let y = if ((s >> 1) & 1) == 1 {
                        tp.y
                    } else {
                        fp.y
                    };
                    let z = if ((s >> 2) & 1) == 1 {
                        tp.z + 1
                    } else {
                        fp.z - 1
                    };
                    targets[use_bots[s]] = Some(Pn { x, y, z });
                }
                waiting.insert((use_bots, (*fp, *tp)));
            }

            let arrived = self.arrived_bots(&targets);
            let arrived = arrived.iter().collect::<HashSet<_>>();

            let (mut commands, mut volatile_set) = self.calc_next_commands(&targets)?;

            let mut remove_list = Vec::new();
            for (bots, (fp, tp)) in &waiting {
                if bots.iter().all(|idx| arrived.contains(idx)) {
                    let mut region = fp.region(*tp);
                    let ok = region.iter().all(|p| !volatile_set.contains(p));
                    //eprintln!("fp = {:?}", fp);
                    //eprintln!("tp = {:?}", tp);
                    for idx in bots {
                        //eprintln!("{:?}", self.game.bots()[*idx]);
                    }
                    if ok {
                        remove_list.push((bots.clone(), (fp.clone(), tp.clone())));
                        volatile_set.extend(region);
                        for s in 0..8 {
                            let idx = bots[s];
                            let dp = if ((s >> 2) & 1) == 1 {
                                PnDiff {
                                    dx: 0,
                                    dy: 0,
                                    dz: -1,
                                }
                            } else {
                                PnDiff {
                                    dx: 0,
                                    dy: 0,
                                    dz: 1,
                                }
                            };
                            let from = self.game.bots()[idx]
                                .pos
                                .add(dp, self.game.resolution())
                                .ok_or("invalid")?;
                            let to = Pn {
                                x: if from.x == fp.x {
                                    tp.x
                                } else {
                                    fp.x
                                },
                                y: if from.y == fp.y {
                                    tp.y
                                } else {
                                    fp.y
                                },
                                z: if from.z == fp.z {
                                    tp.z
                                } else {
                                    fp.z
                                },
                            };
                            let fp = PnDiff::from_i16(to.diff(from));
                            commands[idx] = Command::GVoid(dp, fp);
                            targets[idx] = None;
                        }
                    }
                }
            }
            for item in remove_list {
                waiting.remove(&item);
            }

            self.exec_all(commands)?;
        }
        Ok(())
    }

    fn solve_dis(&mut self) -> Result<(Trace, u64)> {
        self.setup_all()?;

        self.exec_all_remove()?;
        self.exec_remaining_points()?;

        self.finish_all()?;
        Ok((Trace::new(self.commands.clone()), self.game.energy()))
    }

    fn exec_remaining_points(&mut self) -> Result<()> {
        let mut points_by_bots = self.calc_points_by_bots();

        loop {
            let target: Vec<Option<Pn>> =
                points_by_bots.iter().map(|p| p.front().cloned()).collect();

            let arrived = self.arrived_bots(&target);
            for idx in arrived {
                points_by_bots[idx].pop_front();
            }

            let finish = points_by_bots.iter().all(|q| q.is_empty());
            if finish {
                break;
            }

            let target = points_by_bots.iter().map(|p| p.front().cloned()).collect();

            let cmds: Vec<_> = self.calc_next_commands(&target)?.0;

            self.exec_all(cmds)?;
        }
        Ok(())
    }

    fn solve_all(&mut self) -> Result<(Trace, u64)> {
        self.setup_all()?;

        self.exec_remaining_points()?;

        self.finish_all()?;

        Ok((Trace::new(self.commands.clone()), self.game.energy()))
    }

    fn move_single(&self, c: Pn, t: Pn) -> Result<Vec<Command>> {
        let mut res = Vec::new();
        if c == t {
            return Ok(res);
        }
        let p_set = self.game
            .bots()
            .iter()
            .map(|b| b.pos)
            .collect::<HashSet<_>>();
        let mut options = Vec::with_capacity(3);
        let prob = self.prob as u32;
        if c.x < t.x || ((thread_rng().next_u32() % prob == 0) && c.x + 1 < self.game.resolution())
        {
            options.push((1, 0, 0));
        }
        if c.x > t.x || ((thread_rng().next_u32() % prob == 0) && c.x >= 1) {
            options.push((-1, 0, 0));
        };
        if c.y < t.y || ((thread_rng().next_u32() % prob == 0) && c.y + 1 < self.game.resolution())
        {
            options.push((0, 1, 0));
        }
        if c.y > t.y || ((thread_rng().next_u32() % prob == 0) && c.y >= 1) {
            options.push((0, -1, 0));
        }
        if c.z < t.z || ((thread_rng().next_u32() % prob == 0) && c.z + 1 < self.game.resolution())
        {
            options.push((0, 0, 1));
        }
        if c.z > t.z || ((thread_rng().next_u32() % prob == 0) && c.z >= 1) {
            options.push((0, 0, -1));
        }
        let options = options
            .into_iter()
            .filter(|(dx, dy, dz)| {
                if let Some(p) = &c.add(
                    PnDiff {
                        dx: *dx,
                        dy: *dy,
                        dz: *dz,
                    },
                    self.game.resolution(),
                ) {
                    !p_set.contains(p)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        if options.is_empty() {
            return Ok(vec![Command::Wait]);
        }

        //let (dx, dy, dz) = options[0];
        let (dx, dy, dz) = *thread_rng().choose(&options).unwrap();

        let dp = PnDiff { dx, dy, dz };
        let nc = c.add(dp, self.game.resolution()).ok_or("invalid pos")?;

        if !self.game.is_full(nc) && !self.game.must_full(c) {
            let mut k = 1;
            while k < 15 {
                let (f, t, d) = if dx != 0 {
                    (c.x, t.x, dx)
                } else if dy != 0 {
                    (c.y, t.y, dy)
                } else {
                    (c.z, t.z, dz)
                };
                if ((f as i16 + d as i16 * k as i16) - t as i16).abs()
                    < ((f as i16 + d as i16 * (k + 1) as i16) - t as i16).abs()
                {
                    break;
                }
                let ndp = PnDiff {
                    dx: (k + 1) * dx,
                    dy: (k + 1) * dy,
                    dz: (k + 1) * dz,
                };
                let nnc = if let Some(p) = c.add(ndp, self.game.resolution()) {
                    p
                } else {
                    break;
                };

                if self.game.is_full(nnc) {
                    break;
                }
                if p_set.contains(&nnc) {
                    break;
                }
                k += 1;
            }
            let dp = PnDiff {
                dx: k * dx,
                dy: k * dy,
                dz: k * dz,
            };
            res.push(Command::SMove(dp));
        } else {
            if self.game.is_full(nc) {
                res.push(Command::Void(dp));
            }
            res.push(Command::SMove(dp));

            if self.game.must_full(c) {
                let inv_dp = PnDiff {
                    dx: -dx,
                    dy: -dy,
                    dz: -dz,
                };
                res.push(Command::Fill(inv_dp));
            }
        }
        Ok(res)
    }
}

pub fn solve(model_src: &Model, model_dst: &Model) -> Result<Trace> {
    (0..50)
        .map(|_| {
            let prob = thread_rng().next_u32() % 180 + 20;
            if model_src.len() == 0 {
                let mut s = Solver::new(model_src, model_dst, prob as usize, MAX_SEED, 1);
                s.solve_all()
            } else if model_dst.len() == 0 {
                let mut s = Solver::new(model_src, model_dst, prob as usize, 8, 1);
                s.solve_dis()
            } else {
                let mut s = Solver::new(model_src, model_dst, prob as usize, MAX_SEED, 1);
                s.solve_all()
            }
        })
        .enumerate()
        .filter_map(|(i, s)| s.ok().map(|s| (i, s)))
        .take(20)
        .inspect(|(i, (s, energy))| {
            //eprintln!("s.len() = {} energy = {} i = {}", s.len(), energy, i)
        })
        .min_by_key(|(_, (s, c))| c.clone())
        .ok_or("no solution found".into())
        .map(|(_, (s, _))| s)
}

fn enumerate_remove_box(model: &Model) -> Result<Vec<(Pn, Pn)>> {
    let mut res = Vec::new();
    let r = model.resolution();
    let mut matrix = Matrix::from_model(&model);
    let mut remaining = matrix.len();
    let mut removed = 0;
    for fx in 0..r {
        for fy in 0..r {
            for fz in 0..r {
                let fp = Pn {
                    x: fx,
                    y: fy,
                    z: fz,
                };
                if matrix.get(fp) {
                    let tx = cmp::min(r as usize - 1, fx as usize + 30) as u8;
                    let ty = cmp::min(r as usize - 1, fy as usize + 30) as u8;
                    let tz = cmp::min(r as usize - 2, fz as usize + 30) as u8;
                    let size = cmp::min(
                        tx as i16 - fx as i16,
                        cmp::min(ty as i16 - fy as i16, tz as i16 - fz as i16),
                    );
                    let mut count = 0;
                    for x in fx..=tx {
                        for y in fy..=ty {
                            for z in fz..=tz {
                                if matrix.get(Pn { x, y, z }) {
                                    count += 1;
                                }
                            }
                        }
                    }
                    if size <= 3 || count <= 10 {
                        // TODO: parameter !!!
                        // TODO: this is need?
                        continue;
                    }
                    let tp = Pn {
                        x: tx,
                        y: ty,
                        z: tz,
                    };
                    for x in fx..=tx {
                        for y in fy..=ty {
                            for z in fz..=tz {
                                if matrix.get(Pn { x, y, z }) {
                                    matrix.unset(Pn { x, y, z });
                                    remaining -= 1;
                                    removed += 1;
                                }
                            }
                        }
                    }
                    res.push((fp, tp));
                }
            }
        }
    }
    //eprintln!("remaining = {}", remaining);
    //eprintln!("removed = {}", removed);
    Ok(res)
}
