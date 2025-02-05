//! # Neuron Engine

pub mod os;

#[cfg(target_os="linux")]
pub extern crate x11_dl;

#[cfg(windows)]
pub extern crate windows;

pub struct Engine {

}