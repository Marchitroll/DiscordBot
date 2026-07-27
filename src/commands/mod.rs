mod general;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![general::ping(), general::echo(), general::http()]
}
