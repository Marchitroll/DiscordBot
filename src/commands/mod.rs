pub mod confession;
mod general;
pub mod interactions;
pub mod setup;
pub mod tictactoe;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        general::ping(),
        general::echo(),
        general::http(),
        confession::confesion(),
        setup::confesiones_setup(),
        setup::confesiones_deshacer_setup(),
        setup::deshacer_setup(),
        setup::confesiones_config(),
        tictactoe::tictactoe(),
    ]
}
