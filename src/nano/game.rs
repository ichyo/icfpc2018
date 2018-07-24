use nano::*;
use std::cmp;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Debug)]
pub struct State {
    energy: u64,
    harmonics: Harmonics,
    matrix: Matrix,
    bots: Vec<Bot>,
    grounded: Matrix, // TODO: put in game?
}

impl State {
    pub fn new(model_src: &Model) -> State {
        let r = model_src.resolution();
        State {
            energy: 0,
            harmonics: Harmonics::Low,
            matrix: Matrix::from_model(model_src),
            bots: vec![Bot {
                bid: 1,
                pos: Pn::zero(),
                seeds: (2..41).collect(),
            }],
            grounded: Matrix::new(r),
        }
    }
    pub fn is_low(&self) -> bool {
        self.harmonics == Harmonics::Low
    }

    pub fn is_high(&self) -> bool {
        self.harmonics == Harmonics::High
    }

    pub fn is_full(&self, p: Pn) -> bool {
        self.matrix.get(p)
    }

    pub fn set_full(&mut self, p: Pn) -> bool {
        // TODO: Update grounded for unset
        /*
        let mut queue = VecDeque::new();
        if p.y == 0
            || p.adjacents(self.matrix.resolution())
                .any(|np| self.grounded.get(np))
        {
            queue.push_back(p);
            self.grounded.set(p);
        }

        while let Some(p) = queue.pop_front() {
            for np in p.adjacents(self.matrix.resolution()) {
                if self.matrix.get(np) && !self.grounded.get(np) {
                    queue.push_back(np);
                    self.grounded.set(np);
                }
            }
        }
        */

        self.matrix.set(p)
    }

    pub fn set_void(&mut self, p: Pn) -> bool {
        self.matrix.unset(p)
    }

    pub fn flip_harmonics(&mut self) {
        let next = match self.harmonics {
            Harmonics::Low => Harmonics::High,
            Harmonics::High => Harmonics::Low,
        };
        self.harmonics = next;
    }

    fn is_grounded(&self) -> bool {
        self.grounded.len() == self.matrix.len()
    }

    fn check_unreachable_state(&self) -> Result<()> {
        let bid_set = self.bots.iter().map(|b| b.bid).collect::<HashSet<_>>();
        let pos_set = self.bots.iter().map(|b| b.pos).collect::<HashSet<_>>();
        if bid_set.len() != self.bots.len() {
            return Err("bid is not distinct".into());
        }
        if pos_set.len() != self.bots.len() {
            return Err("pos is not distinct".into());
        }
        if pos_set.into_iter().any(|p| self.matrix.get(p)) {
            return Err("bot is at Full Point".into());
        }

        let mut seed_set = HashSet::new();
        for bot in &self.bots {
            for s in &bot.seeds {
                if seed_set.contains(&s) {
                    return Err("seeds is not distinct".into());
                }
                seed_set.insert(s);
            }
        }
        for bid in bid_set {
            if seed_set.contains(&bid) {
                return Err("bid is in another seed set".into());
            }
        }

        Ok(())
    }

    fn check_well_formed(&self) -> Result<()> {
        /*
        if self.harmonics == Harmonics::Low && !self.is_grounded() {
            return Err("harmonics is low but it's not grounded".into());
        }
        */

        // TODO: can be removed for performance
        self.check_unreachable_state()?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Bot {
    pub bid: u8,
    pub pos: Pn,
    pub seeds: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harmonics {
    Low,
    High,
}

#[derive(Clone, Debug)]
pub struct Game<'a> {
    state: State,
    complete: bool,
    model_src: &'a Model,
    model_dst: &'a Model,
    turn: usize,
}

impl<'a> Game<'a> {
    pub fn new(model_src: &'a Model, model_dst: &'a Model) -> Game<'a> {
        Game {
            state: State::new(model_src),
            complete: false,
            model_src,
            model_dst,
            turn: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn is_full(&self, p: Pn) -> bool {
        self.state.is_full(p)
    }

    pub fn must_full(&self, p: Pn) -> bool {
        self.model_dst.get(p)
    }

    pub fn resolution(&self) -> u8 {
        self.state.matrix.resolution()
    }

    pub fn bots(&self) -> &Vec<Bot> {
        &self.state.bots
    }

    pub fn diff_points(&self) -> Vec<Pn> {
        self.model_dst.diff_points_m(&self.state.matrix)
    }

    // make sure we can correspond trace to bot
    fn check_trace_and_bot(&self, trace: &Trace) -> Result<()> {
        if trace.len() != self.state.bots.len() {
            return Err(format!(
                "trace is not correct size {:?} {:?}",
                trace, self.state.bots
            ).into());
        }

        for v in self.state.bots.windows(2) {
            if v[0].bid >= v[1].bid {
                return Err("bid is not sorted".into());
            }
        }

        Ok(())
    }

    fn check_interfare(&self, trace: &Trace) -> Result<()> {
        let mut volatile_set = HashSet::new();
        for (b, t) in self.state.bots.iter().zip(trace.iter()) {
            let mut ps = t.volatile_points(b.pos, self.resolution())?;
            if let Command::GFill(_, f) = t {
                if f.dx < 0 || f.dy < 0 || f.dz < 0 {
                    // skip for uniqueness
                    ps = vec![b.pos];
                }
            }
            if let Command::GVoid(_, f) = t {
                if f.dx < 0 || f.dy < 0 || f.dz < 0 {
                    // skip for uniqueness
                    ps = vec![b.pos];
                }
            }
            for p in &ps {
                if volatile_set.contains(p) {
                    return Err(format!("bots are interfare: {:?} {:?} {:?}", b, t, p).into());
                }
            }
            //eprintln!("{:?} {:?} {:?}", b, t, ps);
            volatile_set.extend(ps);
        }
        Ok(())
    }

    fn check_trace_error(&self, trace: &Trace) -> Result<()> {
        let mut fusion_p_to = HashMap::new();
        let mut fusion_s_to = HashMap::new();
        let mut groups = HashMap::new();
        for (b, t) in self.state.bots.iter().zip(trace.iter()) {
            //eprintln!("{:?} {:?}", b, t);
            match t {
                Command::Halt => {
                    if b.pos != Pn::zero() {
                        return Err("[Halt] not zero pos".into());
                    }
                    if self.state.bots.len() != 1 {
                        return Err("[Halt] not one bot".into());
                    }
                    if self.state.is_high() {
                        return Err("[Halt] harmonics is high".into());
                    }
                    if !self.model_dst.is_complete(&self.state.matrix) {
                        return Err("[Halt] matrix is not complete".into());
                    }
                }
                Command::Wait => {}
                Command::Flip => {}
                Command::SMove(d) => {
                    let t = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;

                    if !d.is_long_linear() {
                        return Err(format!("[SMove] {:?} is not long linear ({:?})", d, b).into());
                    }

                    for p in b.pos.region(t) {
                        if self.state.is_full(p) {
                            return Err(format!("[SMove] {:?} is Full ({:?})", p, b).into());
                        }
                    }
                }
                Command::LMove(d1, d2) => {
                    let f = b.pos;
                    let m = f.add(*d1, self.resolution()).ok_or("invalid pos")?;
                    let t = m.add(*d2, self.resolution()).ok_or("invalid pos")?;

                    if !d1.is_short_linear() {
                        return Err(format!("[LMove] {:?} is not long linear ({:?})", d1, b).into());
                    }
                    if !d2.is_short_linear() {
                        return Err(format!("[LMove] {:?} is not long linear ({:?})", d2, b).into());
                    }

                    for p in [&f.region(m)[..], &m.region(t)[..]].concat() {
                        if self.state.is_full(p) {
                            return Err(format!("[LMove] {:?} is Full", p).into());
                        }
                    }
                }
                Command::Fission(d, m) => {
                    let t = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    if self.state.is_full(t) {
                        return Err(format!("[Fission] {:?} is Full", t).into());
                    }
                    if b.seeds.len() == 0 {
                        return Err("[Fission] seed is empty".into());
                    }
                    if b.seeds.len() <= *m as usize {
                        return Err(
                            "[Fission] m+1 (the size of new parent seed) is larger than n".into(),
                        );
                    }
                    if !d.is_near() {
                        return Err(format!("[Fission] {:?} is not near ({:?})", d, b).into());
                    }
                }
                Command::Fill(d) => {
                    let _ = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    if !d.is_near() {
                        return Err(format!("[Fill] {:?} is not near ({:?})", d, b).into());
                    }
                }
                Command::Void(d) => {
                    let _ = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    if !d.is_near() {
                        return Err(format!("[Void] {:?} is not near ({:?})", d, b).into());
                    }
                }
                Command::FusionP(d) => {
                    let t = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    fusion_p_to.insert(b.pos, t);
                    if !d.is_near() {
                        return Err(format!("[FusionP] {:?} is not near ({:?})", d, b).into());
                    }
                }
                Command::FusionS(d) => {
                    let t = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    fusion_s_to.insert(b.pos, t);
                    if !d.is_near() {
                        return Err(format!("[FusioSP] {:?} is not near ({:?})", d, b).into());
                    }
                }
                Command::GFill(d, f) => {
                    if !d.is_near() {
                        return Err(format!("[GFill] {:?} is not near ({:?})", d, b).into());
                    }
                    if !f.is_far() {
                        return Err(format!("[GFill] {:?} is not far ({:?})", d, f).into());
                    }
                    let fp = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    let tp = fp.add(*f, self.resolution()).ok_or("invalid pos")?;
                    let root = Pn {
                        x: cmp::min(fp.x, tp.x),
                        y: cmp::min(fp.y, tp.y),
                        z: cmp::min(fp.z, tp.z),
                    };
                    *groups.entry(root).or_insert(0) += 1;
                }
                Command::GVoid(d, f) => {
                    if !d.is_near() {
                        return Err(format!("[GFill] {:?} is not near ({:?})", d, b).into());
                    }
                    if !f.is_far() {
                        return Err(format!("[GFill] {:?} is not far ({:?})", d, f).into());
                    }
                    let fp = b.pos.add(*d, self.resolution()).ok_or("invalid pos")?;
                    let tp = fp.add(*f, self.resolution()).ok_or("invalid pos")?;
                    let root = Pn {
                        x: cmp::min(fp.x, tp.x),
                        y: cmp::min(fp.y, tp.y),
                        z: cmp::min(fp.z, tp.z),
                    };
                    *groups.entry(root).or_insert(0) += 1;
                }
            }
        }
        for (from, to) in &fusion_p_to {
            if !fusion_s_to.contains_key(&to) {
                return Err("[FusionP] not matching to FusionS".into());
            }
            if fusion_s_to[to] != *from {
                return Err("[FusionP] matching to diffent place".into());
            }
        }
        for (from, to) in &fusion_s_to {
            if !fusion_p_to.contains_key(&to) {
                return Err("[FusionS] not matching to FusionS".into());
            }
            if fusion_p_to[to] != *from {
                return Err("[FusionS] matching to diffent place".into());
            }
        }
        for (_, v) in groups {
            if v != 8 {
                return Err(format!("[Group] size is not 8 : {}", v).into());
            }
        }
        Ok(())
    }

    fn execute_halt(&mut self, bot: Bot) -> Result<Bot> {
        self.complete = true;
        Ok(bot)
    }

    fn execute_wait(&mut self, bot: Bot) -> Result<Bot> {
        Ok(bot)
    }

    fn execute_flip(&mut self, bot: Bot) -> Result<Bot> {
        self.state.flip_harmonics();
        Ok(bot)
    }

    fn execute_smove(&mut self, mut bot: Bot, d: PnDiff) -> Result<Bot> {
        bot.pos = bot.pos.add(d, self.resolution()).ok_or("invalid pos")?;
        self.state.energy += 2 * d.mlen() as u64;
        Ok(bot)
    }

    fn execute_lmove(&mut self, mut bot: Bot, d1: PnDiff, d2: PnDiff) -> Result<Bot> {
        bot.pos = bot.pos.add(d1, self.resolution()).ok_or("invalid pos")?;
        bot.pos = bot.pos.add(d2, self.resolution()).ok_or("invalid pos")?;
        self.state.energy += 2 * (d1.mlen() as u64 + 2 + d2.mlen() as u64);
        Ok(bot)
    }

    fn execute_fill(&mut self, bot: Bot, d: PnDiff) -> Result<Bot> {
        let p = bot.pos.add(d, self.resolution()).ok_or("invalid pos")?;
        if !self.state.is_full(p) {
            self.state.set_full(p);
            self.state.energy += 12;
        } else {
            self.state.energy += 6;
        }
        Ok(bot)
    }

    fn execute_void(&mut self, bot: Bot, d: PnDiff) -> Result<Bot> {
        let p = bot.pos.add(d, self.resolution()).ok_or("invalid pos")?;
        if self.state.is_full(p) {
            self.state.set_void(p);
            self.state.energy -= 12;
        } else {
            self.state.energy += 3;
        }
        Ok(bot)
    }

    fn execute_fission(&mut self, mut bot: Bot, d: PnDiff, m: u8) -> Result<Vec<Bot>> {
        let new_bot = Bot {
            bid: bot.seeds[0],
            seeds: bot.seeds[1..m as usize + 1].to_owned(),
            pos: bot.pos.add(d, self.resolution()).ok_or("invalid pos")?,
        };
        bot.seeds = if m as usize + 2 < bot.seeds.len() {
            bot.seeds[m as usize + 2..].to_owned()
        } else {
            vec![]
        };
        self.state.energy += 24;
        Ok(vec![bot, new_bot])
    }

    fn execute_fusion(&mut self, mut b1: Bot, b2: Bot) -> Result<Bot> {
        b1.seeds.push(b2.bid);
        b1.seeds.extend(b2.seeds);
        b1.seeds.sort();
        self.state.energy -= 24;
        Ok(b1)
    }

    fn execute_gfill(&mut self, b: Vec<Bot>, b1: Bot, d1: PnDiff, f1: PnDiff) -> Result<Vec<Bot>> {
        let c = b1.pos;
        let pf = c.add(d1, self.resolution()).ok_or("invalid pos")?;
        let pt = pf.add(f1, self.resolution()).ok_or("invalid pos")?;
        let rs = pf.region(pt);
        for p in rs {
            if !self.state.is_full(p) {
                self.state.set_full(p);
                self.state.energy += 12;
            } else {
                self.state.energy += 6;
            }
        }
        Ok(b)
    }
    fn execute_gvoid(&mut self, b: Vec<Bot>, b1: Bot, d1: PnDiff, f1: PnDiff) -> Result<Vec<Bot>> {
        let c = b1.pos;
        let pf = c.add(d1, self.resolution()).ok_or("invalid pos")?;
        let pt = pf.add(f1, self.resolution()).ok_or("invalid pos")?;
        let rs = pf.region(pt);
        for p in rs {
            if self.state.is_full(p) {
                self.state.set_void(p);
                self.state.energy -= 12;
            } else {
                self.state.energy += 3;
            }
        }
        Ok(b)
    }

    fn check_all_errors(&self, trace: &Trace) -> Result<()> {
        self.state.check_well_formed()?;
        self.check_trace_and_bot(trace)?;

        self.check_interfare(trace)?;
        self.check_trace_error(trace)?;

        Ok(())
    }

    fn create_groups<'b>(
        &self,
        bots: &'b Vec<Bot>,
        trace: &'b Trace,
    ) -> Result<HashMap<Pn, VecDeque<(&'b Bot, Command)>>> {
        let mut groups = HashMap::new();
        for (b, t) in bots.iter().zip(trace.iter().cloned()) {
            match t {
                Command::FusionS(d) => {
                    let to = b.pos.add(d, self.resolution()).ok_or("invalid")?;
                    groups
                        .entry(to)
                        .or_insert(VecDeque::with_capacity(2))
                        .push_back((b, t));
                }
                Command::GVoid(d, f) | Command::GFill(d, f) => {
                    let fp = b.pos.add(d, self.resolution()).ok_or("invalid")?;
                    let tp = fp.add(f, self.resolution()).ok_or("invalid")?;
                    let base = Pn {
                        x: cmp::min(fp.x, tp.x),
                        y: cmp::min(fp.y, tp.y),
                        z: cmp::min(fp.z, tp.z),
                    };
                    groups
                        .entry(base)
                        .or_insert(VecDeque::with_capacity(8))
                        .push_back((b, t));
                }
                _ => {
                    groups
                        .entry(b.pos)
                        .or_insert(VecDeque::with_capacity(2))
                        .push_front((b, t));
                }
            }
        }
        Ok(groups)
    }

    fn execute_commands(
        &mut self,
        groups: HashMap<Pn, VecDeque<(&Bot, Command)>>,
    ) -> Result<Vec<Bot>> {
        let mut new_bots: Vec<Bot> = Vec::new();
        for group in groups.values() {
            match group.len() {
                1 => {
                    let (b, t) = group[0];
                    let b = b.clone();
                    match t {
                        Command::Fission(d, m) => {
                            new_bots.extend(self.execute_fission(b, d, m)?);
                        }
                        Command::FusionP(_)
                        | Command::FusionS(_)
                        | Command::GFill(_, _)
                        | Command::GVoid(_, _) => {
                            return Err("unreachable state with wrong group length".into());
                        }
                        _ => {
                            let new_bot = match t {
                                Command::Halt => self.execute_halt(b)?,
                                Command::Wait => self.execute_wait(b)?,
                                Command::Flip => self.execute_flip(b)?,
                                Command::SMove(d) => self.execute_smove(b, d)?,
                                Command::LMove(d1, d2) => self.execute_lmove(b, d1, d2)?,
                                Command::Fill(d) => self.execute_fill(b, d)?,
                                Command::Void(d) => self.execute_void(b, d)?,
                                Command::Fission(_, _)
                                | Command::FusionP(_)
                                | Command::FusionS(_)
                                | Command::GFill(_, _)
                                | Command::GVoid(_, _) => unreachable!(),
                            };
                            new_bots.push(new_bot);
                        }
                    }
                }
                2 => {
                    let (b1, t1) = group[0].clone();
                    let (b2, t2) = group[1].clone();
                    let b1 = b1.clone();
                    let b2 = b2.clone();
                    match (t1, t2) {
                        (Command::FusionP(_), Command::FusionS(_)) => {
                            new_bots.push(self.execute_fusion(b1, b2)?);
                        }
                        _ => {
                            return Err("unreachable state with order of groups".into());
                        }
                    }
                }
                8 => {
                    let (b1, t1) = group[0].clone();
                    let bs = group.iter().map(|(b, _)| (*b).clone()).collect();
                    for (_, t) in group.iter() {
                        match t {
                            Command::GFill(_, _) => {}
                            Command::GVoid(_, _) => {}
                            _ => {
                                return Err(format!(
                                    "unreachable state with different command: {:?}",
                                    t
                                ).into());
                            }
                        }
                    }
                    match t1 {
                        Command::GFill(d, f) => {
                            new_bots.extend(self.execute_gfill(bs, b1.clone(), d, f)?);
                        }
                        Command::GVoid(d, f) => {
                            new_bots.extend(self.execute_gvoid(bs, b1.clone(), d, f)?);
                        }
                        _ => unreachable!(),
                    }
                }
                _ => {
                    return Err("unreachable state with group length".into());
                }
            }
        }

        new_bots.sort_by_key(|b| b.bid);
        Ok(new_bots)
    }

    pub fn energy(&self) -> u64 {
        self.state.energy
    }

    pub fn turn(&self) -> usize {
        self.turn
    }

    pub fn turn_cost(&self) -> u64 {
        let mut res = 0;
        let k = if self.state.is_high() { 30 } else { 3 };
        let r = self.resolution() as u64;
        let n = self.state.bots.len();
        res += k * r * r * r;
        res += 20 * n as u64;
        res
    }

    pub fn execute(&mut self, trace: &Trace) -> Result<()> {
        self.check_all_errors(trace)?;

        self.state.energy += self.turn_cost();

        // TODO: try inplace edit if it's peformance bottleneck.
        let bots = self.state.bots.clone();
        let groups = self.create_groups(&bots, trace)?;
        let new_bots = self.execute_commands(groups)?;

        self.state.bots = new_bots;

        self.turn += 1;

        Ok(())
    }
}
