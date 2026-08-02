pub mod confession;
mod general;
pub mod interactions;
pub mod setup;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        general::ping(),
        general::echo(),
        general::http(),
        confession::confesion(),
        setup::confesiones_setup(),
    ]
}
