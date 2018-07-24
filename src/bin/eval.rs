extern crate csv;
extern crate icfpc2018;
extern crate reqwest;

use icfpc2018::nano::*;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

fn simulate(model_src: Model, model_dst: Model, trace: Trace) -> Result<u64> {
    let mut game = Game::new(&model_src, &model_dst);
    let mut cmd_que = trace.iter().collect::<VecDeque<_>>();

    while cmd_que.len() > 0 {
        let mut cmds = Vec::new();
        for _ in 0..game.bots().len() {
            cmds.push(*cmd_que.pop_front().ok_or("unexpected no entry")?);
        }
        game.execute(&Trace::new(cmds))?;
    }

    if !game.is_complete() {
        return Err("game is not complete".into());
    }

    Ok(game.energy())
}

fn load_model<P: AsRef<Path>>(path: P) -> Result<Model> {
    let file = File::open(path)?;
    let mut buf = BufReader::new(file);
    let model = Model::read(&mut buf)?;
    Ok(model)
}

fn load_trace<P: AsRef<Path>>(path: P) -> Result<Trace> {
    let file = File::open(path)?;
    let mut buf = BufReader::new(file);
    let trace = Trace::read(&mut buf)?;
    Ok(trace)
}

fn load_base<P: AsRef<Path>>(path: P) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf.parse::<u64>()?)
}

fn load_models(id: &str) -> Result<(Model, Model)> {
    if id.starts_with("FA") {
        load_models_FA(id)
    } else if id.starts_with("FD") {
        load_models_FD(id)
    } else if id.starts_with("FR") {
        load_models_FR(id)
    } else {
        unreachable!()
    }
}

#[allow(non_snake_case)]
fn load_models_FA(id: &str) -> Result<(Model, Model)> {
    let model_dst_path = format!("./dataF/{}_tgt.mdl", id);
    let model_dst = load_model(model_dst_path)?;
    let r = model_dst.resolution();
    let model_src = Model::new(r);
    Ok((model_src, model_dst))
}

#[allow(non_snake_case)]
fn load_models_FD(id: &str) -> Result<(Model, Model)> {
    let model_src_path = format!("./dataF/{}_src.mdl", id);
    let model_src = load_model(model_src_path)?;
    let r = model_src.resolution();
    let model_dst = Model::new(r);
    Ok((model_src, model_dst))
}

#[allow(non_snake_case)]
fn load_models_FR(id: &str) -> Result<(Model, Model)> {
    let model_src_path = format!("./dataF/{}_src.mdl", id);
    let model_src = load_model(model_src_path)?;
    let model_dst_path = format!("./dataF/{}_tgt.mdl", id);
    let model_dst = load_model(model_dst_path)?;
    Ok((model_src, model_dst))
}

fn eval(dir: &str, id: &str) -> Result<(u64, u64)> {
    let trace_path = format!("./{}/{}.nbt", dir, id);
    let trace = load_trace(trace_path)?;
    let (model_src, model_dst) = load_models(id)?;
    let energy = simulate(model_src, model_dst, trace)?;
    let base_energy_path = format!("./dataF/{}.base", id);
    let base = load_base(base_energy_path)?;
    Ok((energy, base))
}

fn parse_csv() -> Result<Vec<(String, (u64, u64))>> {
    let url = "https://raw.githubusercontent.com/icfpcontest2018/icfpcontest2018.github.io/master/_data/full_standings_live.csv";
    let body = reqwest::get(url)?.text()?;
    let mut rdr = csv::Reader::from_reader(body.as_bytes());
    let mut min_energy: HashMap<String, (u64, u64)> = HashMap::new();
    for result in rdr.records() {
        let record = result?;
        let (id, energy, score) = (
            &record[2],
            record[3].parse::<u64>().unwrap(),
            record[4].parse::<u64>().unwrap(),
        );
        if id == "total" {
            continue;
        }
        if !min_energy.contains_key(id) || (min_energy.get(id).unwrap().0 > energy) {
            min_energy.insert(id.to_owned(), (energy, score));
        }
    }
    let mut res = min_energy.into_iter().collect::<Vec<_>>();
    res.sort();
    Ok(res)
}

fn calc_score(energy: u64, base: u64, best: u64, max_score: u64) -> (f64, u64) {
    let diff_your = if base >= energy { base - energy } else { 0 };
    let diff_best = if base >= best { base - best } else { 0 };
    let ratio = diff_your as f64 / diff_best as f64;
    (ratio, (ratio * max_score as f64) as u64)
}

fn main() -> Result<()> {
    let dir = env::args().nth(1).unwrap();
    let m = parse_csv()?;
    let mut total_score = 0;
    let mut total_ratio = 0.0;
    let n = m.len();
    for (id, (best, max_score)) in m {
        let (energy, base) = eval(&dir, &id)?;
        println!("--- {} ---", id);
        //println!("best = {}", best);
        //println!("your = {}", energy);
        //println!("base = {}", base);
        println!("ratio = {:.2}", energy as f64 / best as f64);
        let (diff_ratio, score) = calc_score(energy, base, best, max_score);
        println!("diff_ratio = {:.2}", diff_ratio);
        total_score += score;
        total_ratio += diff_ratio;
    }
    println!("total_score = {}", total_score);
    println!("ave_ratio = {}", total_ratio / n as f64);
    Ok(())
}
