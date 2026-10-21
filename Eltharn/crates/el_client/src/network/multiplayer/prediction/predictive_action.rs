#![allow(dead_code)]

pub trait PredictiveAction {
    type Packet;
    fn predict(&self, sequence: i32) -> Self::Packet;
}
