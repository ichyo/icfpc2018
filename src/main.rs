extern crate icfpc2018;
extern crate rayon;

use icfpc2018::nano::*;
use rayon::prelude::*;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

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

fn save_trace<P: AsRef<Path>>(path: P, trace: &Trace) -> Result<()> {
    let file = File::create(path)?;
    let mut buf = BufWriter::new(file);
    trace.write(&mut buf)?;
    Ok(())
}

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

fn load_base<P: AsRef<Path>>(path: P) -> Result<u64> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content.parse::<u64>()?)
}

#[allow(non_snake_case)]
fn load_model_FA(id: usize) -> Result<(Model, Model)> {
    let model_dst_path = format!("./dataF/FA{:03}_tgt.mdl", id);
    let model_dst = load_model(model_dst_path)?;
    let r = model_dst.resolution();
    let model_src = Model::new(r);
    Ok((model_src, model_dst))
}

#[allow(non_snake_case)]
fn load_model_FD(id: usize) -> Result<(Model, Model)> {
    let model_src_path = format!("./dataF/FD{:03}_src.mdl", id);
    let model_src = load_model(model_src_path)?;
    let r = model_src.resolution();
    let model_dst = Model::new(r);
    Ok((model_src, model_dst))
}

#[allow(non_snake_case)]
fn load_model_FR(id: usize) -> Result<(Model, Model)> {
    let model_src_path = format!("./dataF/FR{:03}_src.mdl", id);
    let model_src = load_model(model_src_path)?;
    let model_dst_path = format!("./dataF/FR{:03}_tgt.mdl", id);
    let model_dst = load_model(model_dst_path)?;
    Ok((model_src, model_dst))
}

#[allow(non_snake_case)]
fn simulate_FA(id: usize) -> Result<()> {
    let trace_path = format!("./dataF/FA{:03}.nbt", id);
    let trace = load_trace(trace_path)?;
    let (model_src, model_dst) = load_model_FA(id)?;
    let energy = simulate(model_src, model_dst, trace)?;
    let base_energy_path = format!("./dataF/FA{:03}.base", id);
    write!(File::create(base_energy_path)?, "{}", energy)?;
    Ok(())
}

#[allow(non_snake_case)]
fn simulate_FD(id: usize) -> Result<()> {
    let trace_path = format!("./dataF/FD{:03}.nbt", id);
    let trace = load_trace(trace_path)?;
    let (model_src, model_dst) = load_model_FD(id)?;
    let energy = simulate(model_src, model_dst, trace)?;
    let base_energy_path = format!("./dataF/FD{:03}.base", id);
    write!(File::create(base_energy_path)?, "{}", energy)?;
    Ok(())
}

#[allow(non_snake_case)]
fn simulate_FR(id: usize) -> Result<()> {
    let trace_path = format!("./dataF/FR{:03}.nbt", id);
    let trace = load_trace(trace_path)?;
    let (model_src, model_dst) = load_model_FR(id)?;
    let energy = simulate(model_src, model_dst, trace)?;
    let base_energy_path = format!("./dataF/FR{:03}.base", id);
    write!(File::create(base_energy_path)?, "{}", energy)?;
    Ok(())
}

#[allow(non_snake_case)]
fn solve_FA(id: usize) -> Result<f64> {
    let (model_src, model_dst) = load_model_FA(id)?;
    let answer = solve(&model_src, &model_dst)?;
    let answer_save_path = format!("./answer/FA{:03}.nbt", id);
    save_trace(answer_save_path, &answer)?;
    let energy = simulate(model_src, model_dst, answer)?;

    let base = load_base(format!("./dataF/FA{:03}.base", id))?;
    let ratio = energy as f64 / base as f64;
    println!("--- FA {} ---", id);
    println!("energy = {}", energy);
    println!("ratio = {:.2}%", ratio * 100.0);
    Ok(ratio)
}

#[allow(non_snake_case)]
fn solve_FD(id: usize) -> Result<f64> {
    let (model_src, model_dst) = load_model_FD(id)?;
    let answer = match solve(&model_src, &model_dst) {
        Ok(ans) => ans,
        Err(e) => {
            println!("Failure on {}", id);
            return Err(e);
        }
    };
    let answer_save_path = format!("./answer/FD{:03}.nbt", id);
    save_trace(answer_save_path, &answer)?;
    let energy = simulate(model_src, model_dst, answer)?;

    let base = load_base(format!("./dataF/FD{:03}.base", id))?;
    let ratio = energy as f64 / base as f64;
    println!("--- FD {} ---", id);
    println!("energy = {}", energy);
    println!("ratio = {:.2}%", ratio * 100.0);

    Ok(ratio)
}

#[allow(non_snake_case)]
fn solve_FR(id: usize) -> Result<f64> {
    let (model_src, model_dst) = load_model_FR(id)?;
    let answer = solve(&model_src, &model_dst)?;
    let answer_save_path = format!("./answer/FR{:03}.nbt", id);
    save_trace(answer_save_path, &answer)?;
    let energy = simulate(model_src, model_dst, answer)?;

    let base = load_base(format!("./dataF/FR{:03}.base", id))?;
    let ratio = energy as f64 / base as f64;
    println!("--- FR {} ---", id);
    println!("energy = {}", energy);
    println!("ratio = {:.2}%", ratio * 100.0);

    Ok(ratio)
}

const MAX_FA: usize = 186;
const MAX_FD: usize = 186;
const MAX_FR: usize = 115;

fn simulate_all() -> Result<()> {
    for i in 1..MAX_FA + 1 {
        simulate_FA(i)?;
    }
    for i in 1..MAX_FD + 1 {
        simulate_FD(i)?;
    }
    for i in 1..MAX_FR + 1 {
        simulate_FR(i)?;
    }
    Ok(())
}

fn solve_all() -> Result<()> {
    let mut ratios = Vec::new();
    ratios.extend((1..MAX_FA + 1)
        .collect::<Vec<usize>>()
        .par_iter()
        .map(|i| solve_FA(*i))
        .collect::<Result<Vec<_>>>()?);
    ratios.extend((1..MAX_FD + 1)
        .collect::<Vec<usize>>()
        .par_iter()
        .map(|i| solve_FD(*i))
        .collect::<Result<Vec<_>>>()?);
    ratios.extend((1..MAX_FR + 1)
        .collect::<Vec<usize>>()
        .par_iter()
        .map(|i| solve_FR(*i))
        .collect::<Result<Vec<_>>>()?);
    println!(
        "average = {:.2}%",
        ratios.iter().sum::<f64>() / ratios.len() as f64 * 100.0
    );
    Ok(())
}

fn main() -> Result<()> {
    //simulate_all()?;
    solve_all()?;
    Ok(())
}
