mod game;
mod model;
mod point;
mod trace;
mod util;
mod solver;

pub use self::game::{Bot, Game, State};
pub use self::model::{Matrix, Model};
pub use self::point::{Pn, PnDiff};
pub use self::trace::{Command, Trace};
pub use self::solver::solve;
use std::result;
use std::error::Error;

pub type Result<T> = result::Result<T, Box<Error + Send + Sync>>;
