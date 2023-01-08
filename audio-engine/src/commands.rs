use crate::track::Track;

#[derive(Debug)]
pub enum Command {
    AddTrack(Track),
}
